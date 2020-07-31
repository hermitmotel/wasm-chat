use js_sys::Reflect;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{
    console, window, Document, Event, EventTarget, FormData, HtmlFormElement, HtmlTextAreaElement,
    WebSocket,
};

// When the `wee_alloc` feature is enabled, this uses `wee_alloc` as the global
// allocator.
//
// If you don't want to use `wee_alloc`, you can safely delete this.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

fn setup_ws_connection() -> WebSocket {
    let ws = WebSocket::new_with_str("ws://localhost:2794", "rust-websocket")
        .expect("WebSocket failed to connect 'ws://localhost:2794'");

    let ws_c = ws.clone();
    let open_handler = Box::new(move || {
        console::log_1(&"Send me messages!".into());
    });
    let cb_oh: Closure<dyn Fn()> = Closure::wrap(open_handler);
    ws.set_onopen(Some(cb_oh.as_ref().unchecked_ref()));
    cb_oh.forget();
    ws
}

fn setup_ws_msg_recv(ws: WebSocket) {
    let msg_recv_handler = Box::new(move |msg: JsValue| {
        let window = window().expect("should have a window in this context");
        let document = window.document().expect("window should have a document");
        let textarea = document
            .get_element_by_id("chat-display")
            .expect("No #chat-display")
            .dyn_into::<HtmlTextAreaElement>()
            .unwrap();

        let data: JsValue =
            Reflect::get(&msg, &"data".into()).expect("No 'data' field in websocket message!");

        let message = &data.as_string().expect("Field 'data' is not string");
        let value = textarea.value() + "\n" + message;

        textarea.set_value(&value);
    });
    let cb_mrh: Closure<dyn Fn(JsValue)> = Closure::wrap(msg_recv_handler);
    ws.set_onmessage(Some(cb_mrh.as_ref().unchecked_ref()));

    cb_mrh.forget();
}

fn setup_form_handling(ws: WebSocket) {
    let window = window().expect("should have a window in this context");
    let document = window.document().expect("window should have a document");
    let form = document
        .get_element_by_id("chat-controls")
        .expect("#chat-controls not found.");

    let handler = Box::new(move |event: Event| {
        event.prevent_default();
        let data = FormData::new_with_form(
            form.dyn_ref::<HtmlFormElement>()
                .expect("#chat-controls is not HtmlFormElement"),
        );

        if let Some(msg) = message_from_form(data) {
            ws.send_with_str(&msg).expect("Could not send message");
        }
    });
    let cbx: Closure<dyn Fn(Event)> = Closure::wrap(handler);

    document
        .get_element_by_id("chat-controls")
        .expect("should have #chat-controls on the page")
        .dyn_ref::<EventTarget>()
        .expect("#chat-controls must be an `EventTarget`")
        .add_event_listener_with_callback("submit", cbx.as_ref().unchecked_ref())
        .expect("Could not add event listener");

    cbx.forget();

    fn message_from_form(form_data: Result<FormData, JsValue>) -> Option<String> {
        match form_data {
            Ok(form_data) => {
                let user = form_data
                    .get("username")
                    .as_string()
                    .expect("could not read username from form");
                let text = form_data
                    .get("message")
                    .as_string()
                    .expect("could not read message from form");

                Some(user + ": " + &text)
            }
            Err(_) => None,
        }
    }
}

// Called by our JS entry point to run the example.
#[wasm_bindgen(start)]
pub fn run() -> Result<(), JsValue> {
    // This provides better error messages in debug mode.
    // It's disabled in release mode so it doesn't bloat up the file size.
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();

    let ws = setup_ws_connection();
    setup_ws_msg_recv(ws.clone());
    setup_form_handling(ws);

    Ok(())
}
