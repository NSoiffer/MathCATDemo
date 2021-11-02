use yew::prelude::*;
use yew::web_sys::Element;
use wasm_bindgen::JsCast;
use web_sys::{HtmlInputElement};
use yew::services::ConsoleService;
use regex::Regex;
#[macro_use]
extern crate lazy_static;



#[derive(Debug)]
enum Msg {
    SetMathML(String),
    NewMathML,
    SpeechStyle(&'static str),
    SpeechVerbosity(&'static str),
    BrailleCode(&'static str),
    BrailleDisplayAs(&'static str),
}

struct Model {
    // `ComponentLink` is like a reference to a component.
    // It can be used to send messages to the component
    link: ComponentLink<Self>,
    mathml: String,
    display: Html,
    speech_style: &'static str,
    verbosity: &'static str,
    speech: String,
    braille_code: &'static str,
    braille_display_as: &'static str,
    braille: String,
}

static INPUT_MESSAGE: &'static str = "Enter math: use $...$ for TeX, `...` for ASCIIMath, <math>...</math> for MathML\n";
static QUADRATIC_FORMULA: &'static str = r"$x = {-b \pm \sqrt{b^2-4ac} \over 2a}$";

impl Component for Model {
    type Message = Msg;
    type Properties = ();

    fn create(_props: Self::Properties, link: ComponentLink<Self>) -> Self {
        ConsoleService::info(format!("In Create").as_ref());
        Self {
            link,
            mathml: INPUT_MESSAGE.to_string() + QUADRATIC_FORMULA,
            display: Html::VRef(yew::utils::document().create_element("div").unwrap().into()),
            speech_style: "ClearSpeak",
            verbosity: "Verbose",
            speech: String::default(),
            braille_code: "Nemeth",
            braille_display_as: "Dots",
            braille: String::default(),
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        ConsoleService::info(format!("In update: msg: {:?}", msg).as_ref());
        match msg {
            Msg::SetMathML(value) => {
                ConsoleService::info(format!("setting mathml\n{}`", value).as_ref());
                self.mathml = value;
                true
            },
            Msg::NewMathML => {
                lazy_static! {
                    static ref TEX: Regex = Regex::new("(?m)^(?P<start>\\$)(?P<math>.+?)(?P<end>\\$)$").unwrap();
                    static ref ASCIIMATH: Regex = Regex::new("(?m)^(?P<start>`)(?P<math>.+?)(?P<end>`)$").unwrap();
                    static ref MATHML: Regex = Regex::new("(?m)^(?P<start><)(?P<math>.+?)(?P<end>>)$").unwrap();
                };

                // Get the MathML input string, and clear any previous output
                if let Html::VRef(node) = &self.display {
                    let math_str = self.mathml.replace(INPUT_MESSAGE, "").trim().to_string();
                    let mathml;
                    if let Some(caps) = TEX.captures(&math_str) {
                        mathml = Some(string_to_mathml(&caps["math"], "TeX"));
                    } else if let Some(caps) = ASCIIMATH.captures(&math_str) {
                        mathml = Some(string_to_mathml(&caps["math"], "ASCIIMath"));
                    } else if MATHML.is_match(&math_str) {
                        mathml = Some(string_to_mathml(&math_str, "MathML"));
                    } else {
                        mathml = None;
                    };
                
                    let mathjax_html = match mathml {
                        Some(math) => mathml_to_chtml(math),
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
                    }
                    // update the output to include the new MathML
                    // mathjax_update();
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
                true
            },
            Msg::BrailleDisplayAs(text) => {
                self.braille_display_as = text;
                true
            },
        }
    }

    fn change(&mut self, _props: Self::Properties) -> ShouldRender {
        ConsoleService::info(format!("In change").as_ref());
        // Should only return "true" if new properties are different to
        // previously received properties.
        // This component has no properties so we will always return "false".
        false
    }

    fn view(&self) -> Html {
        let on_blur = self.link.callback(move |e: FocusEvent| {
            let target = e.target().expect("Blur Event doesn't have target!");
            Msg::SetMathML(target.unchecked_into::<HtmlInputElement>().value())
        });
        html! {
            <div>
                <h1>{"MathCAT Demo"}</h1>
                <h2>{"MathML"}</h2>
                <textarea id="mathml-input"  rows="20" cols="80" autocorrect="off" onblur={on_blur}
                    placeholder={INPUT_MESSAGE}>
                    {INPUT_MESSAGE.to_string() + QUADRATIC_FORMULA}
                </textarea>
                <br />
                <div>
                <input type="button" value="Generate Speech and Braille" id="render-button"
                    onclick=self.link.callback(|_| Msg::NewMathML) />
                </div>
                <h2>{"Displayed Math (click to navigate)"}</h2>
                <div id="mathml-output" tabstop="-1">{self.display.clone()}</div>
                <h2>{"Speech"}</h2>
                <table role="presentation"><tr>
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
                </tr></table>
                <textarea id="speech" readonly=true rows="3" cols="80" data-hint=""  autocorrect="off">
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

use wasm_bindgen::prelude::*;
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = "ConvertToMathML")]
    pub fn string_to_mathml(mathml: &str, math_format: &str) -> String;

    #[wasm_bindgen(js_name = "ConvertToCHTML")]
    pub fn mathml_to_chtml(mathml: String) -> Element;

    #[wasm_bindgen(js_name = "UpdateDocument")]
    pub fn mathjax_update();
}

fn main() {
    yew::start_app::<Model>();
}