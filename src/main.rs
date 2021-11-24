
use yew::prelude::*;
use yew::web_sys::Element;
// use wasm_bindgen::JsCast;
// use web_sys::{HtmlInputElement};
use regex::Regex;
#[macro_use]
extern crate lazy_static;

use wasm_bindgen::prelude::*;
use cfg_if::cfg_if;
use libmathcat::interface::*;


cfg_if! {
    if #[cfg(feature = "console_log")] {
        fn init_log() {
            use log::Level;
            console_log::init_with_level(Level::Trace).expect("error initializing log");
        }
    } else {
        fn init_log() {}
    }
}

#[macro_use]
extern crate log;

#[derive(Debug)]
enum Msg {
    NewMathML,
    SpeechStyle(&'static str),
    SpeechVerbosity(&'static str),
    BrailleCode(&'static str),
    BrailleDisplayAs(&'static str),
    TTS(&'static str),
}

struct Model {
    // `ComponentLink` is like a reference to a component.
    // It can be used to send messages to the component
    link: ComponentLink<Self>,
    math_string: String,
    display: Html,
    speech_style: &'static str,
    verbosity: &'static str,
    speech: String,
    speak: bool,
    braille_code: &'static str,
    braille_display_as: &'static str,
    braille: String,
    tts: &'static str,
}

static INPUT_MESSAGE: &'static str = "Enter math: use $...$ for TeX, `...` for ASCIIMath, <math>...</math> for MathML\n";
static START_FORMULA: &'static str = r"$x = {-b \pm \sqrt{b^2-4ac} \over 2a}$";
// static START_FORMULA: &'static str = r"$x = {t \over 2a}$";

fn update_speech_and_braille(component: &mut Model) {
    if component.math_string.is_empty() {
        return;
    }
    SetPreference("Verbosity".to_string(), StringOrFloat::AsString(component.verbosity.to_string())).unwrap();
    SetPreference("SpeechStyle".to_string(), StringOrFloat::AsString(component.speech_style.to_string())).unwrap();
    let tts = if component.tts == "Off" {"None"} else {component.tts};
    SetPreference("TTS".to_string(), StringOrFloat::AsString(tts.to_string())).unwrap();
    SetPreference("Bookmark".to_string(), StringOrFloat::AsString("true".to_string())).unwrap();
    let speech = GetSpokenText().unwrap();
    debug!("  speech: {}", speech);
    if component.speak && component.tts != "Off" {
        speak_text(&speech);
    }
    component.speech = speech;

    let mut braille = GetBraille().unwrap();
    if component.braille_display_as == "ASCIIBraille" {
        lazy_static! {
            static ref UNICODE_TO_ASCII: Vec<char> =
                " A1B'K2L@CIF/MSP\"E3H9O6R^DJG>NTQ,*5<-U8V.%[$+X!&;:4\\0Z7(_?W]#Y)=".chars().collect();
        };

        let mut result = String::with_capacity(braille.len());
        for ch in braille.chars() {
            let i = (ch as usize - 0x2800) &0x3F;     // eliminate dots 7 and 8 if present 
            result.push(UNICODE_TO_ASCII[i]);
        }
        braille = result;
    }
    component.braille = braille;

}

impl Component for Model {
    type Message = Msg;
    type Properties = ();

    fn create(_props: Self::Properties, link: ComponentLink<Self>) -> Self {
        debug!("In Create");
        Self {
            link,
            math_string: String::default(),
            display: Html::VRef(yew::utils::document().create_element("div").unwrap().into()),
            speech_style: "ClearSpeak",
            speak: true,
            verbosity: "Verbose",
            speech: String::default(),
            braille_code: "Nemeth",
            braille_display_as: "Dots",
            braille: String::default(),
            tts: "SSML",
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        lazy_static! {
            static ref TEX: Regex = Regex::new("(?m)^(?P<start>\\$)(?P<math>.+?)(?P<end>\\$)$").unwrap();
            static ref ASCIIMATH: Regex = Regex::new("(?m)^(?P<start>`)(?P<math>.+?)(?P<end>`)$").unwrap();
            static ref MATHML: Regex = Regex::new("(?m)^(?P<start><)(?P<math>.+?)(?P<end>>)$").unwrap();
        };

        debug!("In update: msg: {:?}", msg);
        self.speak = true;      // most of the time we want to speak -- true off when we don't
        match msg {
            Msg::NewMathML => {
                // Get the MathML input string, and clear any previous output
                if let Html::VRef(node) = &self.display {
                    let math_str = get_text_of_element("mathml-input");
                    let math_str = math_str.replace(INPUT_MESSAGE, "").trim().to_string();
                    let mut mathml;
                    if let Some(caps) = TEX.captures(&math_str) {
                        mathml = Some(string_to_mathml(&caps["math"], "TeX"));
                    } else if let Some(caps) = ASCIIMATH.captures(&math_str) {
                        mathml = Some(string_to_mathml(&caps["math"], "ASCIIMath"));
                    } else if MATHML.is_match(&math_str) {
                        // Don't need to convert
                        mathml = Some( math_str );
                    } else {
                        mathml = None;
                    };

                    // this adds ids and canonicalizes the MathML
                    if let Some(math) = mathml {
                        // MathJax bug https://github.com/mathjax/MathJax/issues/2805:  newline at end causes MathJaX to hang(!)
                        mathml = Some( SetMathML(math).unwrap().trim_end().to_string() );
                        debug!("MathML with ids: \n{}", mathml.as_ref().unwrap());
                    }

                    let mathjax_html = match mathml {
                        Some(math) => {
                            self.math_string = math.clone();
                            mathml_to_chtml(math)
                        },
                        None => {
                            let span = yew::utils::document().create_element("span").unwrap();
                            span.set_text_content(Some("Unrecognized Math -- use $...$ for TeX, `...` for ASCIIMath, or enter MathML"));
                            span
                        }
                    };
                    node.set_text_content(Some(""));
                    let result = node.append_child(&mathjax_html);
                    if let Err(e) = result {
                        panic!("append_child returned error '{:?}'", e);
                    };
                };
                true
            },
            Msg::SpeechStyle(text) => {
                self.speech_style = text;
                true
            },
            Msg::SpeechVerbosity(text) => {
                self.verbosity = text;
                true
            },
            Msg::BrailleCode(text) => {
                self.braille_code = text;
                self.speak = false;     // no change to speech when changing braille
                true
            },
            Msg::BrailleDisplayAs(text) => {
                self.braille_display_as = text;
                self.speak = false;     // no change to speech when changing braille
                true
            },
            Msg::TTS(text) => {
                self.tts = text;
                true
            },
        };
        update_speech_and_braille(self);
        return true;
    }

    fn change(&mut self, _props: Self::Properties) -> ShouldRender {
        debug!("In change");
        // Should only return "true" if new properties are different to
        // previously received properties.
        // This component has no properties so we will always return "false".
        false
    }

    fn view(&self) -> Html {
        html! {
            <div>
                <h1>{"MathCAT Demo"}</h1>
                <h2>{"MathML"}</h2>
                <textarea id="mathml-input"  rows="5" cols="80" autocorrect="off"
                    placeholder={INPUT_MESSAGE}>
                    {INPUT_MESSAGE.to_string() + START_FORMULA}
                </textarea>
                <br />
                <div>
                <input type="button" value="Generate Speech and Braille" id="render-button"
                    onclick=self.link.callback(|_| Msg::NewMathML) />
                </div>
                <h2>{"Displayed Math (click to navigate)"}</h2>
                <div id="mathml-output" tabstop="-1 contenteditable">{self.display.clone()}</div>
                <h2>{"Speech"}</h2>
                <table id="outer-table" role="presentation"><tr>     // 1x2 outside table
                    <td><table role="presentation"><tr> // 2x3 table on left
                        <td>{"Speech Style:"}</td>
                        <td><input type="radio" id="ClearSpeak" name="speech_style"
                                checked = {self.speech_style == "ClearSpeak"}
                                onclick=self.link.callback(|_| Msg::SpeechStyle("ClearSpeak"))/>
                        <label for="ClearSpeak">{"ClearSpeak"}</label></td>
                        <td><input type="radio" id="SimpleSpeak" name="speech_style" value="SimpleSpeak"
                                checked = {self.speech_style == "SimpleSpeak"}                           
                                onclick=self.link.callback(|_| Msg::SpeechStyle("SimpleSpeak"))/>
                            <label for="SimpleSpeak">{"SimpleSpeak"}</label></td>
                    </tr><tr>
                        <td>{"Speech Verbosity:"}</td>
                        <td><input type="radio" id="Terse" name="verbosity" value="Terse"
                                checked = {self.verbosity == "Terse"}
                                onclick=self.link.callback(|_| Msg::SpeechVerbosity("Terse"))/>
                            <label for="Terse">{"Terse"}</label></td>
                        <td><input type="radio" id="Medium" name="verbosity" value="Medium"
                                checked = {self.verbosity == "Medium"}
                                onclick=self.link.callback(|_| Msg::SpeechVerbosity("Medium"))/>
                            <label for="Medium">{"Medium"}</label></td>
                        <td><input type="radio" id="Verbose" name="verbosity" value="Verbose"
                                checked = {self.verbosity == "Verbose"}
                                onclick=self.link.callback(|_| Msg::SpeechVerbosity("Verbose"))/>
                            <label for="Verbose">{"Verbose"}</label></td>
                    </tr></table></td>
                    <td><table role="presentation"><tr> // 1x2 table on right
                        <td>{"TTS:"}</td>
                        <td><input type="radio" id="Off" name="tts"
                                checked = {self.tts == "Off"}
                                onclick=self.link.callback(|_| Msg::TTS("Off"))/>
                            <label for="Off">{"Off"}</label></td>
                        <td><input type="radio" id="Plain" name="tts"
                                checked = {self.tts == "None"}
                                onclick=self.link.callback(|_| Msg::TTS("None"))/>
                            <label for="Plain">{"Plain"}</label></td>
                        <td><input type="radio" id="SSML" name="tts" value="SSML"
                                checked = {self.tts == "SSML"}                           
                                onclick=self.link.callback(|_| Msg::TTS("SSML"))/>
                            <label for="SSML">{"SSML"}</label></td>
                    </tr> <tr> <td>{"\u{A0}"}</td> // empty row to get alignment right
                    </tr></table></td>
                </tr></table>
                <textarea id="speech" readonly=true rows="3" cols="80" data-hint="" autocorrect="off">
                    {&self.speech}
                </textarea>
                <h2>{"Braille"}</h2>
                <table role="presentation"><tr>
                    <td>{"Braille Settings:"}</td>
                    <td><input type="radio" id="Nemeth" name="braille_setting" checked=true value="Nemeth"
                            onclick=self.link.callback(|_| Msg::BrailleCode("Nemeth"))/>
                        <label for="Nemeth">{"Nemeth"}</label></td>
                    <td><input type="radio" id="UEB" name="braille_setting" value="UEB" disabled=true
                            onclick=self.link.callback(|_| Msg::BrailleCode("UEB"))/>
                        <label for="UEB">{"UEB"}</label></td>
                </tr><tr>
                    <td>{"View Braille As:"}</td>
                    <td><input type="radio" id="Dots" name="view_braille_as" value="Dots"
                            checked = {self.braille_display_as == "Dots"}
                            onclick=self.link.callback(|_| Msg::BrailleDisplayAs("Dots"))/>
                        <label for="Dots">{"Dots"}</label></td>
                    <td><input type="radio" id="ASCIIBraille" name="view_braille_as" value="ASCIIBraille"
                            checked = {self.braille_display_as == "ASCIIBraille"}
                            onclick=self.link.callback(|_| Msg::BrailleDisplayAs("ASCIIBraille"))/>
                        <label for="ASCIIBraille">{"ASCIIBraille"}</label></td>
                </tr></table>
                <textarea id="braille" readonly=true rows="2" cols="80" data-hint="" autocorrect="off">
                    {&self.braille}
                </textarea>
            </div>
        }
    }
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = "ConvertToMathML")]
    pub fn string_to_mathml(mathml: &str, math_format: &str) -> String;

    #[wasm_bindgen(js_name = "ConvertToCHTML")]
    pub fn mathml_to_chtml(mathml: String) -> Element;

    #[wasm_bindgen(js_name = "GetTextOfElement")]
    pub fn get_text_of_element(id: &str) -> String;
    // This is needed because .get_element_by_id("mathml-input") fails in the following code when used where this is called
    // let _foo = Document::new()
    //         .expect("global document not set")
    //     .get_element_by_id("mathml-input")
    //         .expect("element with id `mathml-input` not present")   // this fails (???)
    //     .unchecked_into::<HtmlElement>();

    #[wasm_bindgen(js_name = "SpeakText")]
    pub fn speak_text(text: &str);

    #[wasm_bindgen(js_name = "RustInit")]
    pub fn do_it(text: String);
}

fn main() {
    init_log();
    yew::start_app::<Model>();
    // this is deliberately obscure
    do_it("傮㮼䋝㓿䑖傮⧚ȾჄ╎ℂȾⰸ⧚Ⱦ㎲䑖䋝㞥㮼㣾Ⱦⰸ⧚Ⱦ䣙㙐㣾㮼䑖䋝Ⱦⰸ#ෘ#Ⱦ䶀䩢Ҩ㙐ㄤ䩢䯯ҨܙȾ౺傮㮼䋝㓿䑖傮⧚ȾჄ╎ℂȾⰸ⧚Ⱦ㎲䑖䋝㞥㮼㣾Ⱦⰸ⧚Ⱦ㎲䣙㙐㓿㙐䋝䯯㮼ㄤ㿷䩢Ⱦⰸ#ෘ#䋝㙐傮#Ⴤ╎ℂԝቒ䑖㣾䋝㮼䯯䑖᝜㓿㙐䋝䯯㮼䯯召ቒ䣙㙐㓿㙐䋝䯯㮼ㄤ㿷䩢ʛ场᝜㓿㙐䋝䯯㮼䯯召ᷳ䑖䑖㿷᝜㓿௑#Ⱦ䶀䩢Ҩ㙐ㄤ䩢䯯Ҩܙ௑ㄤ㞥ࣀ࠯ܙڔؓؓҨڔ㙐ॕବҨ࠯ࣀઋ࠯Ҩ㉩ઋࣀㄤҨॕ㎲㎲ܙܙ৮㎲㞥ବ㉩ବܙȾз#媘˼"
            .chars()
            .map(|ch| solve(ch))
            .collect::<String>()
        );

    fn solve(ch: char) -> char {
        let x = ( (65.0 + ((65*65-4*2*(66-(ch as usize))) as f64).sqrt()) / (2.0*2.0) ).round();
        return unsafe { char::from_u32_unchecked(x as u32) }
    }
}
