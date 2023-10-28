
use yew::prelude::*;
use yew::web_sys::Element;
// use wasm_bindgen::JsCast;
// use web_sys::{HtmlInputElement};
use regex::Regex;
#[macro_use]
extern crate lazy_static;

use wasm_bindgen::prelude::*;
use cfg_if::cfg_if;
use libmathcat::*;


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
    // NavMode(&'static str),
    // NavVerbosity(&'static str),
    // SpeechStyle(&'static str),
    // SpeechVerbosity(&'static str),
    // SayCaps(&'static str),
    // BrailleCode(&'static str),
    BrailleDisplayAs(&'static str),
    // TTS(&'static str),
    // Dots(&'static str),
    // Navigate(KeyboardEvent),
}

struct Model {
    // `ComponentLink` is like a reference to a component.
    // It can be used to send messages to the component
    link: ComponentLink<Self>,
    math_string: String,
    display: Html,
    braille_display_as: String,
    nemeth_braille: String,
    ueb_braille: String,
    nemeth_braille_node_ref: NodeRef,
    ueb_braille_node_ref: NodeRef,
    update_braille: bool,
}

impl Model {
    fn save_state(&self) {
        // don't save state for braille demo
    }

    fn init_state_from_cookies(&mut self) {
        let cookies = set_cookie("");
        for cookie in cookies.split(';') {
            let mut key_value = cookie.split('=');
            let key = key_value.next();
            let value = key_value.next();
            match (key, value) {
                (Some(key), Some(value)) => set_state(self, key.trim(), value.trim()),
                _ => (),
            }
        }

        fn set_state(model: &mut Model, name: &str, value: &str) {
            let value = value.to_string();
            match name {
                "braille_display_as" => model.braille_display_as = value,
                _ => (),
            }
        }
    }
}
static INPUT_MESSAGE: &'static str = "Auto-detect format: override using $...$ for TeX, `...` for ASCIIMath, <math>...</math> for MathML\n";
static START_FORMULA: &'static str = r"$x = {-b \pm \sqrt{b^2-4ac} \over 2a}$";
// static START_FORMULA: &'static str = r"$x = {t \over 2a}$";

/// get text for level 1 header
fn get_header() -> String {
    return format!("MathCAT Demo (using v{})", get_version());
}


fn update_speech_and_braille(component: &mut Model) {
    if component.math_string.is_empty() {
        return;
    }

    if component.update_braille {
        component.nemeth_braille  = update_braille_code(component, "Nemeth");
        component.ueb_braille = update_braille_code(component, "UEB");
        component.update_braille = false;
    }

    fn update_braille_code(component: &mut Model, code: &str) -> String {
        set_preference("BrailleCode".to_string(), code.to_string()).unwrap();
        let braille = match get_braille("".to_string()) {
            Ok(str) => str,
            Err(e) => errors_to_string(&e),
        };
        if component.braille_display_as == "ASCIIBraille" {
            lazy_static! {
                static ref UNICODE_TO_ASCII: Vec<char> =
                    " A1B'K2L@CIF/MSP\"E3H9O6R^DJG>NTQ,*5<-U8V.%[$+X!&;:4\\0Z7(_?W]#Y)=".chars().collect();
            };
    
            let mut ascii_braille = String::with_capacity(braille.len());
            for ch in braille.chars() {
                let i = (ch as usize - 0x2800) &0x3F;     // eliminate dots 7 and 8 if present 
                let mut ascii_str = UNICODE_TO_ASCII[i].to_string();
                if ch as usize > 0x283F {
                    ascii_str = format!("<span style='font-weight:bold'>{}</span>", &ascii_str);
                }
                ascii_braille.push_str(&ascii_str);
            }
            return ascii_braille;
        } else {
            return braille;
        }
    }
}

impl Component for Model {
    type Message = Msg;
    type Properties = ();

    fn create(_props: Self::Properties, link: ComponentLink<Self>) -> Self {
        let mut initial_state = Self {
            link,
            math_string: String::default(),
            display: Html::VRef(yew::utils::document().create_element("div").unwrap().into()),
            braille_display_as: "Dots".to_string(),
            nemeth_braille: String::default(),
            ueb_braille: String::default(),
            nemeth_braille_node_ref: NodeRef::default(),
            ueb_braille_node_ref: NodeRef::default(),
            update_braille: true,
        };
        
        initial_state.init_state_from_cookies();
        if set_rules_dir("Rules".to_string()).is_err() {
            panic!("Didn't find rules dir");
        };
        return initial_state;
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        lazy_static! {
            static ref TEX: Regex = Regex::new("(?m)^(?P<start>\\$)(?P<math>.+?)(?P<end>\\$)$").unwrap();
            static ref ASCIIMATH: Regex = Regex::new("(?m)^(?P<start>`)(?P<math>.+?)(?P<end>`)$").unwrap();
            static ref MATHML: Regex = Regex::new("(?m)^(?P<start><)(?P<math>.+?)(?P<end>>)$").unwrap();
        };

        debug!("======= In update: msg: {:?}", msg);
        self.update_braille = false;    // turn on when appropriate
        match msg {
            Msg::NewMathML => {
                // Get the MathML input string, and clear any previous output
                if let Html::VRef(node) = &self.display {
                    let math_str = get_text_of_element("mathml-input");
                    let math_str = math_str.replace(INPUT_MESSAGE, "").replace("\n", " ").trim().to_string();
                    let mut mathml;
                    if let Some(caps) = TEX.captures(&math_str) {
                        debug!("TeX: '{}'", &math_str);
                        mathml = Some(string_to_mathml(&caps["math"], "TeX"));
                    } else if let Some(caps) = ASCIIMATH.captures(&math_str) {
                        mathml = Some(string_to_mathml(&caps["math"], "ASCIIMath"));
                    } else if MATHML.is_match(&math_str) {
                        // Don't need to convert
                        mathml = Some( math_str );
                    } else {
                        // auto-detect -- look for {}'s as a sign it is TeX, otherwise ASCIIMath (which accepts a lot of TeX)
                        mathml = Some(
                            string_to_mathml(
                            &math_str,
                            if math_str.contains("}") {"TeX"} else {"ASCIIMath"}
                            )
                        );
                    };
                    // this adds ids and canonicalizes the MathML
                    if let Some(mut math) = mathml {
                        if !math.contains("display=\"block\"") && !math.contains("display='block'") {
                            math = math.replace("<math ", "<math display='block' ");
                        }
                        // MathJax bug https://github.com/mathjax/MathJax/issues/2805:  newline at end causes MathJaX to hang(!)
                        match set_mathml(math) {
                            Ok(m) => {
                                let math = m.trim_end().to_string();
                                debug!("MathML with ids: \n{}", &math);
                                mathml = Some(math);
                            },
                            Err(e) => {
                                debug!("{}", e);
                                mathml = None;
                            },
                        }
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
                    self.update_braille = true;
                };
            },
            Msg::BrailleDisplayAs(text) => {
                self.braille_display_as = text.to_string();
                self.update_braille = true;
            },
        };
        update_speech_and_braille(self);
        self.save_state();
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
                <h1>{get_header()}</h1>
                <h2 id="math-input-area">{"Math Input Area"}</h2>
                <textarea id="mathml-input" name="math-input-area" rows="4" cols="80" autocorrect="off"
                    placeholder={INPUT_MESSAGE}>
                    {INPUT_MESSAGE.to_string() + START_FORMULA}
                </textarea>
                <br />
                <div>
                <input type="button" value="Generate Speech and Braille" id="render-button"
                    onclick=self.link.callback(|_| Msg::NewMathML) />
                </div>
                <h2 id="math-display-area">{"Displayed Math"}</h2>
                <div role="application" id="mathml-output" tabindex="0" aria-roledescription="displayed math">
                    {self.display.clone()}
                </div>
                <h2 id="braille-heading">{"Braille"}</h2>
                <table role="presentation"><tr>     // 1x2 outside table
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
                    </tr></table>
                <h3 id="nemeth-heading">{"Nemeth"}</h3>
                <div role="region" aria-labelledby="nemeth-heading" id="nemeth_braille" readonly=true tabindex="0" rows="2" cols="80" data-hint="" autocorrect="off"
                    ref={self.nemeth_braille_node_ref.clone()}>
                </div>
                <h3 id="ueb-heading">{"UEB"}</h3>
                <div role="region" aria-labelledby="ueb-heading" id="ueb_braille" readonly=true tabindex="0" rows="2" cols="80" data-hint="" autocorrect="off"
                    ref={self.ueb_braille_node_ref.clone()}>
                </div>
                <p>
                  <a href="https://github.com/NSoiffer/MathCAT/issues" target="_blank" rel="noreferrer">{"Please report bugs here."}</a>
                </p>
            </div>
        }
    }

    fn rendered(&mut self, _first_render: bool) {
        // this allows for bolding of chars in the braille ASCII display
        let el = self.nemeth_braille_node_ref.cast::<Element>().unwrap();
        el.set_inner_html(&self.nemeth_braille);
        let el = self.ueb_braille_node_ref.cast::<Element>().unwrap();
        el.set_inner_html(&self.ueb_braille);
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
    
    #[wasm_bindgen(js_name = "SetCookie")]
    pub fn set_cookie(new_cookie: &str) -> String;
    
    #[wasm_bindgen(js_name = "RustInit")]
    pub fn do_it(text: String);
}


#[wasm_bindgen]
pub fn load_yaml_file(file_name: &str, contents: &str) {
    // for security reasons, only the last component of the name is available. We assume (for debugging) the location
    let file_path = format!("Rules/Languages/en/{}", file_name);
    libmathcat::shim_filesystem::override_file_for_debugging_rules(&file_path, contents);
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
        let x = ( (65.0 + ((65*65-4*2*(66-(ch as isize))) as f64).sqrt()) / (2.0*2.0) ).round();
        return unsafe { char::from_u32_unchecked(x as u32) }
    }
}
