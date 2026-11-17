//! Space is Enter at an option prompt (#3092). TRIM and EXTEND ask for a keyword with a default
//! ("Enter a trim mode option [Quick/Standard] <Quick>:"): a space takes the default, as AutoCAD
//! does, instead of being typed into the command line. Prompts that ask for free text still take
//! spaces.

use cadcraft_engine::Session;
use cadcraft_engine::doc::EntityKind;
use cadcraft_ui_egui::control::{ControlRequest, Outcome, handle};
use cadcraft_ui_egui::{CadApp, Services};
use serde_json::{Value, json};

fn call(app: &mut CadApp, ctx: &egui::Context, method: &str, params: Value) -> Value {
    let (req, _reply) = ControlRequest::new(method, params);
    match handle(app, ctx, &req) {
        Outcome::Done(v) => {
            assert_eq!(v["ok"], true, "{v}");
            v["result"].clone()
        }
        _ => panic!("unexpected screenshot"),
    }
}

fn frame(app: &mut CadApp, ctx: &egui::Context) {
    let mut raw = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 800.0))), ..Default::default() };
    app.raw_input_hook(ctx, &mut raw);
    cadcraft_ui_egui::cmdline::capture_tab(ctx, &mut raw);
    let mut out = ctx.run_ui(raw, |ui| {
        app.logic(ui.ctx());
        app.ui(ui);
    });
    // No renderer here: drop the texture updates deliberately.
    out.textures_delta.clear();
}

/// Run frames until the synthetic input queued with `ui.text`/`ui.key` has reached the app.
fn pump(app: &mut CadApp, ctx: &egui::Context) {
    for _ in 0..3 {
        frame(app, ctx);
    }
    assert!(app.synthetic.is_empty());
    assert!(app.canvas.rect.is_some());
}

fn state(app: &mut CadApp, ctx: &egui::Context) -> Value {
    call(app, ctx, "cmdline.state", json!({}))
}

fn prompt(app: &mut CadApp, ctx: &egui::Context) -> String {
    state(app, ctx)["prompt"].as_str().unwrap_or_default().to_string()
}

/// Type a space at the command line, as the keyboard does, and check it was Enter: nothing is left
/// in the command line and the prompt is back to picking objects.
fn space_is_enter(app: &mut CadApp, ctx: &egui::Context, picking: &str) {
    call(app, ctx, "ui.text", json!({"text": " "}));
    pump(app, ctx);
    let st = state(app, ctx);
    assert_eq!(st["buffer"], "", "the space was typed into the command line: {st}");
    let p = st["prompt"].as_str().unwrap_or_default();
    assert!(p.contains(picking), "the space did not take the option: {p}");
}

#[test]
fn space_takes_the_option_at_the_trim_and_extend_option_prompts() {
    let ctx = egui::Context::default();
    let mut app = CadApp::new(Session::new(), Services::default());
    pump(&mut app, &ctx);

    // TRIM's mode prompt (Quick mode: it starts by picking the object to trim).
    call(&mut app, &ctx, "cmdline.input", json!({"text": "trim"}));
    call(&mut app, &ctx, "cmdline.input", json!({"text": "mo"}));
    assert!(prompt(&mut app, &ctx).contains("Enter a trim mode option"), "{}", prompt(&mut app, &ctx));
    space_is_enter(&mut app, &ctx, "Select object to trim");

    // Its projection prompt.
    call(&mut app, &ctx, "cmdline.input", json!({"text": "project"}));
    assert!(prompt(&mut app, &ctx).contains("Enter a projection option"), "{}", prompt(&mut app, &ctx));
    space_is_enter(&mut app, &ctx, "Select object to trim");
    call(&mut app, &ctx, "cmdline.key", json!({"key": "escape"}));

    // EXTEND asks the same question in its own words.
    call(&mut app, &ctx, "cmdline.input", json!({"text": "extend"}));
    call(&mut app, &ctx, "cmdline.input", json!({"text": "mo"}));
    assert!(prompt(&mut app, &ctx).contains("Enter an extend mode option"), "{}", prompt(&mut app, &ctx));
    space_is_enter(&mut app, &ctx, "Select object to extend");
    call(&mut app, &ctx, "cmdline.key", json!({"key": "escape"}));

    // The edge prompt is offered in Standard mode (TRIMEXTENDMODE 0), which first asks for the
    // cutting edges: Enter selects all of them.
    call(&mut app, &ctx, "engine.execute", json!({"command": "setvar", "params": {"name": "TRIMEXTENDMODE", "value": 0}}));
    call(&mut app, &ctx, "cmdline.input", json!({"text": "trim"}));
    assert!(prompt(&mut app, &ctx).contains("Select objects"), "{}", prompt(&mut app, &ctx));
    call(&mut app, &ctx, "cmdline.input", json!({"text": ""}));
    call(&mut app, &ctx, "cmdline.input", json!({"text": "edge"}));
    assert!(prompt(&mut app, &ctx).contains("Enter an implied edge extension mode"), "{}", prompt(&mut app, &ctx));
    space_is_enter(&mut app, &ctx, "Select object to trim");
}

#[test]
fn space_enters_a_typed_option_at_an_option_prompt() {
    let ctx = egui::Context::default();
    let mut app = CadApp::new(Session::new(), Services::default());
    pump(&mut app, &ctx);
    call(&mut app, &ctx, "cmdline.input", json!({"text": "trim"}));
    call(&mut app, &ctx, "cmdline.input", json!({"text": "mo"}));
    assert!(prompt(&mut app, &ctx).contains("Enter a trim mode option"), "{}", prompt(&mut app, &ctx));
    // S then space is the Standard option: it asks for the cutting edges, as Enter would.
    call(&mut app, &ctx, "ui.text", json!({"text": "s"}));
    pump(&mut app, &ctx);
    call(&mut app, &ctx, "ui.text", json!({"text": " "}));
    pump(&mut app, &ctx);
    let st = state(&mut app, &ctx);
    assert_eq!(st["buffer"], "", "{st}");
    let p = st["prompt"].as_str().unwrap_or_default();
    assert!(p.contains("Select objects"), "S then space is the Standard option: {p}");
}

#[test]
fn space_is_enter_at_lines_point_prompt() {
    // LINE asks for a point, not text, so its space was already Enter: it ends the line.
    let ctx = egui::Context::default();
    let mut app = CadApp::new(Session::new(), Services::default());
    pump(&mut app, &ctx);
    call(&mut app, &ctx, "cmdline.input", json!({"text": "line"}));
    call(&mut app, &ctx, "cmdline.input", json!({"text": "0,0"}));
    assert!(prompt(&mut app, &ctx).contains("Specify next point"), "{}", prompt(&mut app, &ctx));
    call(&mut app, &ctx, "ui.text", json!({"text": " "}));
    pump(&mut app, &ctx);
    let st = state(&mut app, &ctx);
    assert_eq!(st["buffer"], "", "{st}");
    assert!(st["running"].is_null(), "space ends LINE: {st}");
}

#[test]
fn a_prompt_for_free_text_still_takes_spaces() {
    let ctx = egui::Context::default();
    let mut app = CadApp::new(Session::new(), Services::default());
    pump(&mut app, &ctx);
    for text in ["text", "0,0", "0.2", "0"] {
        call(&mut app, &ctx, "cmdline.input", json!({"text": text}));
    }
    assert!(prompt(&mut app, &ctx).contains("Enter text"), "{}", prompt(&mut app, &ctx));
    for word in ["hello", " ", "world"] {
        call(&mut app, &ctx, "ui.text", json!({"text": word}));
        pump(&mut app, &ctx);
    }
    assert_eq!(state(&mut app, &ctx)["buffer"], "hello world");
    call(&mut app, &ctx, "ui.key", json!({"key": "enter"}));
    pump(&mut app, &ctx);
    let texts: Vec<String> = app
        .session
        .doc()
        .unwrap()
        .model
        .iter()
        .filter_map(|e| match &e.kind {
            EntityKind::Text(t) => Some(t.value.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(texts, ["hello world"]);
}
