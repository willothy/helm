use leptos::leptos_dom::ev::SubmitEvent;
use leptos::*;
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::to_value;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "tauri"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

#[derive(Serialize, Deserialize)]
pub struct LoginArgs<'a> {
    username: &'a str,
    password: &'a str,
}

#[component]
pub fn App() -> impl IntoView {
    let (username, set_username) = create_signal(String::new());
    let (password, set_password) = create_signal(String::new());

    let clear_password = move || {
        set_password.set(String::new());
    };
    let update_name = move |name: String| {
        set_username.set(name);
    };

    let login = move |ev: SubmitEvent| {
        ev.prevent_default();

        spawn_local(async move {
            if username.get().is_empty() {
                clear_password();
                return;
            }

            let args = to_value(&LoginArgs {
                username: &username.get(),
                password: &password.get(),
            })
            .unwrap();
            let response = invoke("try_login", args).await;
            let transformed: Result<helm_shared::LoginResult, _> =
                serde_wasm_bindgen::from_value(response);
            update_name(format!("{transformed:?}"));
        });
    };

    view! {
        <main class="container">
            <div class="login-form-wrapper">
                <form class="login-form" on:submit=login>
                    <input
                        placeholder="Username"
                        on:input=move |ev| set_username.set(event_target_value(&ev))
                        prop:value=move|| username.get()
                    />
                    <input
                        // type="password"
                        placeholder="Password"
                        on:input=move |ev| set_password.set(event_target_value(&ev))
                        prop:value=move|| password.get()
                    />
                    <button type="submit">"Login"</button>
                </form>
            </div>
        </main>
    }
}
