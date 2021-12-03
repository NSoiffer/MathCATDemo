
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
    Dots(&'static str),
    Navigate(KeyboardEvent),
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
    nav_id: String,
    braille_code: &'static str,
    braille_display_as: &'static str,
    braille_dots78: &'static str,
    braille: String,
    braille_node_ref: NodeRef,
    tts: &'static str,

    update_speech: bool,
    update_braille: bool,
}

static INPUT_MESSAGE: &'static str = "Enter math: use $...$ for TeX, `...` for ASCIIMath, <math>...</math> for MathML\n";
static START_FORMULA: &'static str = r"$x = {-b \pm \sqrt{b^2-4ac} \over 2a}$";
// static START_FORMULA: &'static str = r"$x = {t \over 2a}$";

fn update_speech_and_braille(component: &mut Model) {
    if component.math_string.is_empty() {
        return;
    }

    if component.update_speech {
        SetPreference("Verbosity".to_string(), StringOrFloat::AsString(component.verbosity.to_string())).unwrap();
        SetPreference("SpeechStyle".to_string(), StringOrFloat::AsString(component.speech_style.to_string())).unwrap();
        let tts = if component.tts == "Off" {"None"} else {component.tts};
        SetPreference("TTS".to_string(), StringOrFloat::AsString(tts.to_string())).unwrap();
        SetPreference("Bookmark".to_string(), StringOrFloat::AsString("true".to_string())).unwrap();
        let speech = GetSpokenText().unwrap();
        component.speech = speech;
        component.speak = true;  
        component.update_speech = false;  
    }

    if component.speak && component.tts != "Off" {
        speak_text(&component.speech);
        component.speak = false;
    }

    if component.update_braille {
        SetPreference("BrailleNavHighlight".to_string(), StringOrFloat::AsString(component.braille_dots78.to_string())).unwrap();
        let mut braille = GetBraille(component.nav_id.clone()).unwrap();
        if component.braille_display_as == "ASCIIBraille" {
            lazy_static! {
                static ref UNICODE_TO_ASCII: Vec<char> =
                    " A1B'K2L@CIF/MSP\"E3H9O6R^DJG>NTQ,*5<-U8V.%[$+X!&;:4\\0Z7(_?W]#Y)=".chars().collect();
            };
    
            let mut result = String::with_capacity(braille.len());
            for ch in braille.chars() {
                let i = (ch as usize - 0x2800) &0x3F;     // eliminate dots 7 and 8 if present 
                let mut ascii_str = UNICODE_TO_ASCII[i].to_string();
                if ch as usize > 0x283F {
                    ascii_str = format!("<span style='font-weight:bold'>{}</span>", &ascii_str);
                }
                result.push_str(&ascii_str);
            }
            braille = result;
        }
        component.braille = braille;    
        component.update_braille = false;
    }
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
            nav_id: String::default(),
            braille_dots78: "EndPoints",
            braille_code: "Nemeth",
            braille_display_as: "Dots",
            braille: String::default(),
            braille_node_ref: NodeRef::default(),
            tts: "Off", // "SSML",

            update_speech: true,
            update_braille: true,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        lazy_static! {
            static ref TEX: Regex = Regex::new("(?m)^(?P<start>\\$)(?P<math>.+?)(?P<end>\\$)$").unwrap();
            static ref ASCIIMATH: Regex = Regex::new("(?m)^(?P<start>`)(?P<math>.+?)(?P<end>`)$").unwrap();
            static ref MATHML: Regex = Regex::new("(?m)^(?P<start><)(?P<math>.+?)(?P<end>>)$").unwrap();
        };

        debug!("In update: msg: {:?}", msg);
        self.update_braille = false;    // turn on when appropriate
        self.update_speech = false;     // turn on when appropriate
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
                    self.nav_id = "".to_string();
                    self.update_braille = true;
                    self.update_speech = true;
                };
                true
            },
            Msg::SpeechStyle(text) => {
                self.speech_style = text;
                self.update_speech = true;
                true
            },
            Msg::SpeechVerbosity(text) => {
                self.verbosity = text;
                self.update_speech = true;
                true
            },
            Msg::BrailleCode(text) => {
                self.braille_code = text;
                self.update_braille = true;
                true
            },
            Msg::BrailleDisplayAs(text) => {
                self.braille_display_as = text;
                self.update_braille = true;
                true
            },
            Msg::TTS(text) => {
                self.tts = text;
                self.update_speech = true;
                true
            },
            Msg::Dots(text) => {
                self.braille_dots78 = text;
                self.update_braille = true;
                true
            },
            Msg::Navigate(ev) => {
                use phf::phf_set;
                static VALID_NAV_KEYS: phf::Set<u32> = phf_set! {
                    /*Enter*/0x0Du32, /*Space*/0x20u32, /*Home*/0x24u32, /*End*/0x23u32, /*Backspace*/0x08u32,
                    /*ArrowDown*/0x28u32,  /*ArrowLeft*/0x25u32,  /*ArrowRight*/0x27u32,  /*ArrowUp*/0x26u32, 
                    /*0-9*/0x30u32, 0x31u32, 0x32u32, 0x33u32, 0x34u32, 0x35u32, 0x36u32, 0x37u32, 0x38u32, 0x39u32, 
                };
                
                debug!("  alt {}, ctrl {}, charCode {}, code {}, key {}, keyCode {}",
                        ev.alt_key(), ev.ctrl_key(), ev.char_code(), ev.code(), ev.key(), ev.key_code());
                // should use ev.code -- KeyJ, ArrowRight, etc
                // however, MathPlayer defined values that match ev.key_code, so we use them
                // for debugging Nav Rules
                if ev.key() == "Pause" {
                    // open a FileReader to read the Nav File so we don't need to recompile
                    get_file();     // this starts the sequence to get the file -- we will get a callback later
                }
                
                if ev.key() == "Escape" {
                    remove_focus("mathml-output");
                } else if VALID_NAV_KEYS.contains(&ev.key_code()) {
                    ev.stop_propagation();
                    ev.prevent_default();    
                    match DoNavigateKeyPress(ev.key_code() as usize, ev.shift_key(), ev.ctrl_key(), ev.alt_key(), ev.meta_key()) {
                        Ok(speech) => {
                            self.speech = speech;
                            let id_and_offset = GetNavigationMathMLId().unwrap();
                            self.nav_id = id_and_offset.0;
                            highlight_nav_element(&self.nav_id);
                            self.speak = true;
                            self.update_braille = true;
                        },
                        Err(e) => {
                            libmathcat::speech::print_errors(&e.chain_err(|| "Navigation failure!"));
                            self.speech = "Error in Navigation (key combo not yet implement?) -- see console log for more info".to_string()
                        },
                    };    
                }
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
                <h2>
                    {"Displayed Math (click to navigate, ESC to exit ["}
                    <a href="https://docs.wiris.com/en/mathplayer/navigation_commands" target="_blank">{"nav help"}</a>
                    {"])"}
                </h2>
                <div role="application" id="mathml-output" tabindex="0" aria-roledescription="navigable displayed math"
                        onkeydown=self.link.callback(|ev| Msg::Navigate(ev))>
                    {self.display.clone()}
                </div>
                <h2 id="speech-heading">{"Speech"}</h2>
                <table role="presentation"><tr>     // 1x2 outside table
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
                <textarea role="application" id="speech" aria-labelledby="speech-heading" readonly=true rows="3" cols="80" data-hint="" autocorrect="off">
                    {&self.speech}
                </textarea>
                <h2 id="braille-heading">{"Braille"}</h2>
                <table role="presentation"><tr>     // 1x2 outside table
                    <td><table role="presentation"><tr>
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
                                <label for="ASCIIBraille">{"ASCIIBraille"}</label>
                        </td>
                    </tr></table></td>
                    <td><table role="presentation"><tr> // 1x2 table on right
                        <td>{"\u{A0}"}</td> // empty row to get alignment right
                        </tr> <tr>
                        <td>{"\u{A0}\u{A0}\u{A0}Navigation Indicator:"}</td>
                        <td><input type="radio" id="DotsOff" name="dots-78"
                                checked = {self.braille_dots78 == "Off"}
                                onclick=self.link.callback(|_| Msg::Dots("Off"))/>
                            <label for="DotsOff">{"Off"}</label></td>
                        <td><input type="radio" id="DotsFirstChar" name="dots-78"
                                checked = {self.braille_dots78 == "FirstChar"}
                                onclick=self.link.callback(|_| Msg::Dots("FirstChar"))/>
                            <label for="DotsFirstChar">{"FirstChar"}</label></td>
                        <td><input type="radio" id="DotsEndPoints" name="dots-78" value="EndPoints"
                                checked = {self.braille_dots78 == "EndPoints"}                           
                                onclick=self.link.callback(|_| Msg::Dots("EndPoints"))/>
                            <label for="DotsEndPoints">{"EndPoints"}</label></td>
                        <td><input type="radio" id="DotsAll" name="dots-78" value="All"
                                checked = {self.braille_dots78 == "All"}                           
                                onclick=self.link.callback(|_| Msg::Dots("All"))/>
                            <label for="DotsAll">{"All"}</label></td>
                    </tr> </table></td>
                </tr></table>
                <p aria-labelledby="braille-heading" id="braille" readonly=true rows="2" cols="80" data-hint="" autocorrect="off"
                    ref={self.braille_node_ref.clone()}>
                </p>
            </div>
        }
    }

    fn rendered(&mut self, _first_render: bool) {
        // this allows for bolding of chars in the braille ASCII display
        let el = self.braille_node_ref.cast::<Element>().unwrap();
        el.set_inner_html(&self.braille);
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

    #[wasm_bindgen(js_name = "HighlightNavigationElement")]
    pub fn highlight_nav_element(text: &str);

    #[wasm_bindgen(js_name = "RemoveFocus")]
    pub fn remove_focus(text: &str);

    #[wasm_bindgen(js_name = "GetFile")]
    pub fn get_file();
    
    #[wasm_bindgen(js_name = "RustInit")]
    pub fn do_it(text: String);
}


#[wasm_bindgen]
pub fn load_yaml_file(contents: &str) {
    debug!("yaml file starts: '{}'", &contents[0..20]);
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
