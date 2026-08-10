use std::collections::HashMap;

use serde_json::Value;

use super::cdp::client::CdpClient;
use super::cdp::types::*;
use super::element::{resolve_element_center, resolve_element_object_id, RefMap};

pub async fn click(
    client: &CdpClient,
    session_id: &str,
    ref_map: &RefMap,
    selector_or_ref: &str,
    button: &str,
    click_count: i32,
    iframe_sessions: &HashMap<String, String>,
) -> Result<(), String> {
    if button == "left" && click_count == 1 {
        let (object_id, effective_session_id) = resolve_element_object_id(
            client,
            session_id,
            ref_map,
            selector_or_ref,
            iframe_sessions,
        )
        .await?;

        let activation_result: Value = client
            .send_command_typed(
                "Runtime.callFunctionOn",
                &CallFunctionOnParams {
                    function_declaration: r#"function() {
                        const el = this.closest?.('a[href], button, input, [role="button"]') || this;
                        const tag = el.tagName ? el.tagName.toUpperCase() : "";
                        const type = el.type ? String(el.type).toLowerCase() : "";
                        const role = el.getAttribute?.("role");
                        const isLink = tag === "A" && !!el.href;
                        const isDomClickTarget =
                            tag === "BUTTON" ||
                            (tag === "INPUT" && ["button", "submit", "reset", "checkbox", "radio"].includes(type)) ||
                            role === "button";

                        if (!isLink && !isDomClickTarget) {
                            return null;
                        }

                        el.focus?.();
                        el.click();
                        return "dom-click";
                    }"#
                    .to_string(),
                    object_id: Some(object_id),
                    arguments: None,
                    return_by_value: Some(true),
                    await_promise: Some(false),
                },
                Some(&effective_session_id),
            )
            .await?;

        let activation_mode = activation_result
            .get("result")
            .and_then(|v| v.get("value"))
            .and_then(|v| v.as_str());

        if activation_mode == Some("dom-click") {
            return Ok(());
        }
    }

    let (x, y, effective_session_id) = resolve_element_center(
        client,
        session_id,
        ref_map,
        selector_or_ref,
        iframe_sessions,
    )
    .await?;
    dispatch_click(client, &effective_session_id, x, y, button, click_count).await
}

pub async fn dblclick(
    client: &CdpClient,
    session_id: &str,
    ref_map: &RefMap,
    selector_or_ref: &str,
    iframe_sessions: &HashMap<String, String>,
) -> Result<(), String> {
    click(
        client,
        session_id,
        ref_map,
        selector_or_ref,
        "left",
        2,
        iframe_sessions,
    )
    .await
}

pub async fn hover(
    client: &CdpClient,
    session_id: &str,
    ref_map: &RefMap,
    selector_or_ref: &str,
    iframe_sessions: &HashMap<String, String>,
) -> Result<(), String> {
    let (x, y, effective_session_id) = resolve_element_center(
        client,
        session_id,
        ref_map,
        selector_or_ref,
        iframe_sessions,
    )
    .await?;
    client
        .send_command_typed::<_, Value>(
            "Input.dispatchMouseEvent",
            &DispatchMouseEventParams {
                event_type: "mouseMoved".to_string(),
                x,
                y,
                button: None,
                buttons: None,
                click_count: None,
                delta_x: None,
                delta_y: None,
                modifiers: None,
            },
            Some(&effective_session_id),
        )
        .await?;
    Ok(())
}

pub async fn fill(
    client: &CdpClient,
    session_id: &str,
    ref_map: &RefMap,
    selector_or_ref: &str,
    value: &str,
    iframe_sessions: &HashMap<String, String>,
) -> Result<(), String> {
    let (object_id, effective_session_id) = resolve_element_object_id(
        client,
        session_id,
        ref_map,
        selector_or_ref,
        iframe_sessions,
    )
    .await?;

    // Focus the element
    client
        .send_command_typed::<_, Value>(
            "Runtime.callFunctionOn",
            &CallFunctionOnParams {
                function_declaration: "function() { this.focus(); }".to_string(),
                object_id: Some(object_id.clone()),
                arguments: None,
                return_by_value: Some(true),
                await_promise: Some(false),
            },
            Some(&effective_session_id),
        )
        .await?;

    // Select all + delete to clear
    client
        .send_command_typed::<_, Value>(
            "Runtime.callFunctionOn",
            &CallFunctionOnParams {
                function_declaration: r#"function() {
                    this.select && this.select();
                    this.value = '';
                    this.dispatchEvent(new Event('input', { bubbles: true }));
                }"#
                .to_string(),
                object_id: Some(object_id),
                arguments: None,
                return_by_value: Some(true),
                await_promise: Some(false),
            },
            Some(&effective_session_id),
        )
        .await?;

    // Insert text (keyboard input dispatched at page level, use parent session_id)
    client
        .send_command_typed::<_, Value>(
            "Input.insertText",
            &InsertTextParams {
                text: value.to_string(),
            },
            Some(session_id),
        )
        .await?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn type_text(
    client: &CdpClient,
    session_id: &str,
    ref_map: &RefMap,
    selector_or_ref: &str,
    text: &str,
    clear: bool,
    delay_ms: Option<u64>,
    iframe_sessions: &HashMap<String, String>,
) -> Result<(), String> {
    let (object_id, effective_session_id) = resolve_element_object_id(
        client,
        session_id,
        ref_map,
        selector_or_ref,
        iframe_sessions,
    )
    .await?;

    // Focus
    client
        .send_command_typed::<_, Value>(
            "Runtime.callFunctionOn",
            &CallFunctionOnParams {
                function_declaration: "function() { this.focus(); }".to_string(),
                object_id: Some(object_id.clone()),
                arguments: None,
                return_by_value: Some(true),
                await_promise: Some(false),
            },
            Some(&effective_session_id),
        )
        .await?;

    if clear {
        client
            .send_command_typed::<_, Value>(
                "Runtime.callFunctionOn",
                &CallFunctionOnParams {
                    function_declaration: r#"function() {
                        this.select && this.select();
                        this.value = '';
                        this.dispatchEvent(new Event('input', { bubbles: true }));
                    }"#
                    .to_string(),
                    object_id: Some(object_id),
                    arguments: None,
                    return_by_value: Some(true),
                    await_promise: Some(false),
                },
                Some(&effective_session_id),
            )
            .await?;
    }

    type_text_into_active_context(client, session_id, text, delay_ms).await
}

pub async fn type_text_into_active_context(
    client: &CdpClient,
    session_id: &str,
    text: &str,
    delay_ms: Option<u64>,
) -> Result<(), String> {
    let delay = delay_ms.unwrap_or(0);

    for ch in text.chars() {
        if matches!(ch, '\n' | '\r' | '\t') {
            let (key, code, key_code) = char_to_key_info(ch);
            let text_str = key_text(&key);
            client
                .send_command_typed::<_, Value>(
                    "Input.dispatchKeyEvent",
                    &DispatchKeyEventParams {
                        event_type: "keyDown".to_string(),
                        key: Some(key.clone()),
                        code: Some(code.clone()),
                        text: text_str.clone(),
                        unmodified_text: text_str,
                        windows_virtual_key_code: Some(key_code),
                        native_virtual_key_code: Some(key_code),
                        modifiers: None,
                    },
                    Some(session_id),
                )
                .await?;

            client
                .send_command_typed::<_, Value>(
                    "Input.dispatchKeyEvent",
                    &DispatchKeyEventParams {
                        event_type: "keyUp".to_string(),
                        key: Some(key),
                        code: Some(code),
                        text: None,
                        unmodified_text: None,
                        windows_virtual_key_code: Some(key_code),
                        native_virtual_key_code: Some(key_code),
                        modifiers: None,
                    },
                    Some(session_id),
                )
                .await?;
        } else {
            // VS Code/Electron webviews reject repeated dispatchKeyEvent calls
            // carrying printable `text`. Insert printable characters directly
            // and reserve key events for controls like Enter and Tab.
            client
                .send_command_typed::<_, Value>(
                    "Input.insertText",
                    &InsertTextParams {
                        text: ch.to_string(),
                    },
                    Some(session_id),
                )
                .await?;
        }

        if delay > 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
        }
    }

    Ok(())
}

pub async fn press_key(client: &CdpClient, session_id: &str, key: &str) -> Result<(), String> {
    press_key_with_modifiers(client, session_id, key, None).await
}

/// Dispatch a keyDown+keyUp sequence for `key` with an optional CDP modifier bitmask.
///
/// Modifier values follow the CDP `Input.dispatchKeyEvent` spec:
/// 1 = Alt, 2 = Control, 4 = Meta (Cmd), 8 = Shift.
///
/// Callers that need a platform-appropriate modifier (e.g. Cmd on macOS,
/// Ctrl elsewhere) must choose the value themselves -- see `cfg!(target_os)`.
pub async fn press_key_with_modifiers(
    client: &CdpClient,
    session_id: &str,
    key: &str,
    modifiers: Option<i32>,
) -> Result<(), String> {
    let (key_name, code, key_code) = named_key_info(key);

    // Suppress text insertion when Control (2) or Meta (4) modifiers are active,
    // since these are command chords (e.g. Ctrl+A = select-all), not text input.
    let has_command_modifier = modifiers.is_some_and(|m| m & (2 | 4) != 0);
    let text = if has_command_modifier {
        None
    } else {
        key_text(&key_name)
    };

    client
        .send_command_typed::<_, Value>(
            "Input.dispatchKeyEvent",
            &DispatchKeyEventParams {
                event_type: "keyDown".to_string(),
                key: Some(key_name.clone()),
                code: Some(code.clone()),
                text: text.clone(),
                unmodified_text: text.clone(),
                windows_virtual_key_code: Some(key_code),
                native_virtual_key_code: Some(key_code),
                modifiers,
            },
            Some(session_id),
        )
        .await?;

    client
        .send_command_typed::<_, Value>(
            "Input.dispatchKeyEvent",
            &DispatchKeyEventParams {
                event_type: "keyUp".to_string(),
                key: Some(key_name),
                code: Some(code),
                text: None,
                unmodified_text: None,
                windows_virtual_key_code: Some(key_code),
                native_virtual_key_code: Some(key_code),
                modifiers,
            },
            Some(session_id),
        )
        .await?;

    Ok(())
}

pub async fn scroll(
    client: &CdpClient,
    session_id: &str,
    ref_map: &RefMap,
    selector_or_ref: Option<&str>,
    delta_x: f64,
    delta_y: f64,
    iframe_sessions: &HashMap<String, String>,
) -> Result<(), String> {
    if let Some(sel) = selector_or_ref {
        let (object_id, effective_session_id) =
            resolve_element_object_id(client, session_id, ref_map, sel, iframe_sessions).await?;
        let js = "function(dx, dy) { this.scrollBy(dx, dy); }".to_string();
        client
            .send_command_typed::<_, Value>(
                "Runtime.callFunctionOn",
                &CallFunctionOnParams {
                    function_declaration: js,
                    object_id: Some(object_id),
                    arguments: Some(vec![
                        CallArgument {
                            value: Some(serde_json::json!(delta_x)),
                            object_id: None,
                        },
                        CallArgument {
                            value: Some(serde_json::json!(delta_y)),
                            object_id: None,
                        },
                    ]),
                    return_by_value: Some(true),
                    await_promise: Some(false),
                },
                Some(&effective_session_id),
            )
            .await?;
    } else {
        let js = format!("window.scrollBy({}, {})", delta_x, delta_y);
        client
            .send_command_typed::<_, Value>(
                "Runtime.evaluate",
                &EvaluateParams {
                    expression: js,
                    return_by_value: Some(true),
                    await_promise: Some(false),
                },
                Some(session_id),
            )
            .await?;
    }
    Ok(())
}

pub async fn select_option(
    client: &CdpClient,
    session_id: &str,
    ref_map: &RefMap,
    selector_or_ref: &str,
    values: &[String],
    iframe_sessions: &HashMap<String, String>,
) -> Result<(), String> {
    let (object_id, effective_session_id) = resolve_element_object_id(
        client,
        session_id,
        ref_map,
        selector_or_ref,
        iframe_sessions,
    )
    .await?;

    let js = r#"function(vals) {
            const options = Array.from(this.options);
            for (const opt of options) {
                opt.selected = vals.includes(opt.value) || vals.includes(opt.textContent.trim());
            }
            this.dispatchEvent(new Event('change', { bubbles: true }));
        }"#
    .to_string();

    client
        .send_command_typed::<_, Value>(
            "Runtime.callFunctionOn",
            &CallFunctionOnParams {
                function_declaration: js,
                object_id: Some(object_id),
                arguments: Some(vec![CallArgument {
                    value: Some(serde_json::json!(values)),
                    object_id: None,
                }]),
                return_by_value: Some(true),
                await_promise: Some(false),
            },
            Some(&effective_session_id),
        )
        .await?;

    Ok(())
}

pub async fn check(
    client: &CdpClient,
    session_id: &str,
    ref_map: &RefMap,
    selector_or_ref: &str,
    iframe_sessions: &HashMap<String, String>,
) -> Result<(), String> {
    let is_checked = super::element::is_element_checked(
        client,
        session_id,
        ref_map,
        selector_or_ref,
        iframe_sessions,
    )
    .await?;
    if !is_checked {
        click(
            client,
            session_id,
            ref_map,
            selector_or_ref,
            "left",
            1,
            iframe_sessions,
        )
        .await?;

        // Verify the click changed the state (Playwright parity: _setChecked re-checks).
        // If the coordinate-based click missed (e.g. hidden input, overlay), retry
        // with a JS .click() on the element and its associated input.
        if !super::element::is_element_checked(
            client,
            session_id,
            ref_map,
            selector_or_ref,
            iframe_sessions,
        )
        .await?
        {
            js_click_checkbox(
                client,
                session_id,
                ref_map,
                selector_or_ref,
                iframe_sessions,
            )
            .await?;
        }
    }
    Ok(())
}

pub async fn uncheck(
    client: &CdpClient,
    session_id: &str,
    ref_map: &RefMap,
    selector_or_ref: &str,
    iframe_sessions: &HashMap<String, String>,
) -> Result<(), String> {
    let is_checked = super::element::is_element_checked(
        client,
        session_id,
        ref_map,
        selector_or_ref,
        iframe_sessions,
    )
    .await?;
    if is_checked {
        click(
            client,
            session_id,
            ref_map,
            selector_or_ref,
            "left",
            1,
            iframe_sessions,
        )
        .await?;

        // Same verify-and-retry as check().
        if super::element::is_element_checked(
            client,
            session_id,
            ref_map,
            selector_or_ref,
            iframe_sessions,
        )
        .await?
        {
            js_click_checkbox(
                client,
                session_id,
                ref_map,
                selector_or_ref,
                iframe_sessions,
            )
            .await?;
        }
    }
    Ok(())
}

/// Fallback for when the coordinate-based CDP click did not toggle the
/// checkbox/radio state. This mirrors how Playwright dispatches clicks
/// through the DOM rather than via raw Input.dispatchMouseEvent coordinates.
///
/// Uses the same follow-label resolution as `is_element_checked`:
/// 1. If the element is a native input → `.click()` it directly.
/// 2. If the element is inside a `<label>` → `.click()` the label's `.control`.
/// 3. If the element has a nested `<input>` → `.click()` that input.
/// 4. Otherwise → `.click()` the element itself (handles ARIA role controls).
async fn js_click_checkbox(
    client: &CdpClient,
    session_id: &str,
    ref_map: &RefMap,
    selector_or_ref: &str,
    iframe_sessions: &HashMap<String, String>,
) -> Result<(), String> {
    let (object_id, effective_session_id) = resolve_element_object_id(
        client,
        session_id,
        ref_map,
        selector_or_ref,
        iframe_sessions,
    )
    .await?;

    let js = r#"function() {
            var el = this;
            var tag = el.tagName && el.tagName.toUpperCase();
            // 1. Native input — click it directly
            if (tag === 'INPUT' && (el.type === 'checkbox' || el.type === 'radio')) {
                el.click();
                return;
            }
            // 2. Follow label → control association
            var label = tag === 'LABEL' ? el : (el.closest && el.closest('label'));
            if (label && label.tagName && label.tagName.toUpperCase() === 'LABEL' && label.control) {
                label.control.click();
                return;
            }
            // 3. Nested native input
            var input = el.querySelector && el.querySelector('input[type="checkbox"], input[type="radio"]');
            if (input) {
                input.click();
                return;
            }
            // 4. ARIA role control — click the element itself
            el.click();
        }"#;

    client
        .send_command_typed::<_, Value>(
            "Runtime.callFunctionOn",
            &CallFunctionOnParams {
                function_declaration: js.to_string(),
                object_id: Some(object_id),
                arguments: None,
                return_by_value: Some(true),
                await_promise: Some(false),
            },
            Some(&effective_session_id),
        )
        .await?;

    Ok(())
}

pub async fn focus(
    client: &CdpClient,
    session_id: &str,
    ref_map: &RefMap,
    selector_or_ref: &str,
    iframe_sessions: &HashMap<String, String>,
) -> Result<(), String> {
    let (object_id, effective_session_id) = resolve_element_object_id(
        client,
        session_id,
        ref_map,
        selector_or_ref,
        iframe_sessions,
    )
    .await?;

    client
        .send_command_typed::<_, Value>(
            "Runtime.callFunctionOn",
            &CallFunctionOnParams {
                function_declaration: "function() { this.focus(); }".to_string(),
                object_id: Some(object_id),
                arguments: None,
                return_by_value: Some(true),
                await_promise: Some(false),
            },
            Some(&effective_session_id),
        )
        .await?;

    Ok(())
}

pub async fn clear(
    client: &CdpClient,
    session_id: &str,
    ref_map: &RefMap,
    selector_or_ref: &str,
    iframe_sessions: &HashMap<String, String>,
) -> Result<(), String> {
    let (object_id, effective_session_id) = resolve_element_object_id(
        client,
        session_id,
        ref_map,
        selector_or_ref,
        iframe_sessions,
    )
    .await?;

    client
        .send_command_typed::<_, Value>(
            "Runtime.callFunctionOn",
            &CallFunctionOnParams {
                function_declaration: r#"function() {
                    this.focus();
                    this.value = '';
                    this.dispatchEvent(new Event('input', { bubbles: true }));
                    this.dispatchEvent(new Event('change', { bubbles: true }));
                }"#
                .to_string(),
                object_id: Some(object_id),
                arguments: None,
                return_by_value: Some(true),
                await_promise: Some(false),
            },
            Some(&effective_session_id),
        )
        .await?;

    Ok(())
}

pub async fn select_all(
    client: &CdpClient,
    session_id: &str,
    ref_map: &RefMap,
    selector_or_ref: &str,
    iframe_sessions: &HashMap<String, String>,
) -> Result<(), String> {
    let (object_id, effective_session_id) = resolve_element_object_id(
        client,
        session_id,
        ref_map,
        selector_or_ref,
        iframe_sessions,
    )
    .await?;

    client
        .send_command_typed::<_, Value>(
            "Runtime.callFunctionOn",
            &CallFunctionOnParams {
                function_declaration: r#"function() {
                    this.focus();
                    if (typeof this.select === 'function') {
                        this.select();
                    } else {
                        const range = document.createRange();
                        range.selectNodeContents(this);
                        const sel = window.getSelection();
                        sel.removeAllRanges();
                        sel.addRange(range);
                    }
                }"#
                .to_string(),
                object_id: Some(object_id),
                arguments: None,
                return_by_value: Some(true),
                await_promise: Some(false),
            },
            Some(&effective_session_id),
        )
        .await?;

    Ok(())
}

pub async fn scroll_into_view(
    client: &CdpClient,
    session_id: &str,
    ref_map: &RefMap,
    selector_or_ref: &str,
    iframe_sessions: &HashMap<String, String>,
) -> Result<(), String> {
    let (object_id, effective_session_id) = resolve_element_object_id(
        client,
        session_id,
        ref_map,
        selector_or_ref,
        iframe_sessions,
    )
    .await?;

    client
        .send_command_typed::<_, Value>(
            "Runtime.callFunctionOn",
            &CallFunctionOnParams {
                function_declaration:
                    "function() { this.scrollIntoView({ block: 'center', inline: 'center' }); }"
                        .to_string(),
                object_id: Some(object_id),
                arguments: None,
                return_by_value: Some(true),
                await_promise: Some(false),
            },
            Some(&effective_session_id),
        )
        .await?;

    Ok(())
}

pub async fn dispatch_event(
    client: &CdpClient,
    session_id: &str,
    ref_map: &RefMap,
    selector_or_ref: &str,
    event_type: &str,
    event_init: Option<&Value>,
    iframe_sessions: &HashMap<String, String>,
) -> Result<(), String> {
    let (object_id, effective_session_id) = resolve_element_object_id(
        client,
        session_id,
        ref_map,
        selector_or_ref,
        iframe_sessions,
    )
    .await?;

    let init_json = event_init
        .map(|v| serde_json::to_string(v).unwrap_or("{}".to_string()))
        .unwrap_or_else(|| "{ bubbles: true }".to_string());

    let js = format!(
        "function() {{ this.dispatchEvent(new Event({}, {})); }}",
        serde_json::to_string(event_type).unwrap_or_default(),
        init_json
    );

    client
        .send_command_typed::<_, Value>(
            "Runtime.callFunctionOn",
            &CallFunctionOnParams {
                function_declaration: js,
                object_id: Some(object_id),
                arguments: None,
                return_by_value: Some(true),
                await_promise: Some(false),
            },
            Some(&effective_session_id),
        )
        .await?;

    Ok(())
}

pub async fn highlight(
    client: &CdpClient,
    session_id: &str,
    ref_map: &RefMap,
    selector_or_ref: &str,
    iframe_sessions: &HashMap<String, String>,
) -> Result<(), String> {
    let (object_id, effective_session_id) = resolve_element_object_id(
        client,
        session_id,
        ref_map,
        selector_or_ref,
        iframe_sessions,
    )
    .await?;

    client
        .send_command_typed::<_, Value>(
            "Runtime.callFunctionOn",
            &CallFunctionOnParams {
                function_declaration: r#"function() {
                    this.style.outline = '2px solid red';
                    this.style.outlineOffset = '2px';
                    const el = this;
                    setTimeout(() => {
                        el.style.outline = '';
                        el.style.outlineOffset = '';
                    }, 3000);
                }"#
                .to_string(),
                object_id: Some(object_id),
                arguments: None,
                return_by_value: Some(true),
                await_promise: Some(false),
            },
            Some(&effective_session_id),
        )
        .await?;

    Ok(())
}

pub async fn tap_touch(
    client: &CdpClient,
    session_id: &str,
    ref_map: &RefMap,
    selector_or_ref: &str,
    iframe_sessions: &HashMap<String, String>,
) -> Result<(), String> {
    let (x, y, effective_session_id) = resolve_element_center(
        client,
        session_id,
        ref_map,
        selector_or_ref,
        iframe_sessions,
    )
    .await?;

    client
        .send_command(
            "Input.dispatchTouchEvent",
            Some(serde_json::json!({
                "type": "touchStart",
                "touchPoints": [{ "x": x, "y": y }],
            })),
            Some(&effective_session_id),
        )
        .await?;

    client
        .send_command(
            "Input.dispatchTouchEvent",
            Some(serde_json::json!({
                "type": "touchEnd",
                "touchPoints": [],
            })),
            Some(&effective_session_id),
        )
        .await?;

    Ok(())
}

async fn dispatch_click(
    client: &CdpClient,
    session_id: &str,
    x: f64,
    y: f64,
    button: &str,
    click_count: i32,
) -> Result<(), String> {
    // Move
    client
        .send_command_typed::<_, Value>(
            "Input.dispatchMouseEvent",
            &DispatchMouseEventParams {
                event_type: "mouseMoved".to_string(),
                x,
                y,
                button: None,
                buttons: None,
                click_count: None,
                delta_x: None,
                delta_y: None,
                modifiers: None,
            },
            Some(session_id),
        )
        .await?;

    let button_value = match button {
        "right" => 2,
        "middle" => 4,
        _ => 1,
    };

    // Press
    client
        .send_command_typed::<_, Value>(
            "Input.dispatchMouseEvent",
            &DispatchMouseEventParams {
                event_type: "mousePressed".to_string(),
                x,
                y,
                button: Some(button.to_string()),
                buttons: Some(button_value),
                click_count: Some(click_count),
                delta_x: None,
                delta_y: None,
                modifiers: None,
            },
            Some(session_id),
        )
        .await?;

    // Release
    client
        .send_command_typed::<_, Value>(
            "Input.dispatchMouseEvent",
            &DispatchMouseEventParams {
                event_type: "mouseReleased".to_string(),
                x,
                y,
                button: Some(button.to_string()),
                buttons: Some(0),
                click_count: Some(click_count),
                delta_x: None,
                delta_y: None,
                modifiers: None,
            },
            Some(session_id),
        )
        .await?;

    Ok(())
}

fn char_to_key_info(ch: char) -> (String, String, i32) {
    match ch {
        '\n' | '\r' => ("Enter".to_string(), "Enter".to_string(), 13),
        '\t' => ("Tab".to_string(), "Tab".to_string(), 9),
        ' ' => (" ".to_string(), "Space".to_string(), 32),
        _ => {
            let key = ch.to_string();
            if ch.is_ascii_alphabetic() {
                // For letters the Windows VK code equals the uppercase ASCII value.
                let upper = ch.to_ascii_uppercase();
                let code = format!("Key{}", upper);
                let key_code = upper as i32;
                (key, code, key_code)
            } else if ch.is_ascii_digit() {
                let code = format!("Digit{}", ch);
                let key_code = ch as i32;
                (key, code, key_code)
            } else {
                let (code, key_code) = punctuation_key_info(ch);
                (key, code.to_string(), key_code)
            }
        }
    }
}

/// Return the DOM `KeyboardEvent.code` value and Windows virtual-key code for
/// a punctuation / symbol character assuming a US keyboard layout.
///
/// The Windows virtual-key codes (VK_OEM_*) differ from ASCII values for
/// punctuation.  Using the raw ASCII code would misidentify characters – e.g.
/// '.' (ASCII 46) collides with VK_DELETE (0x2E = 46), causing the period to
/// be swallowed.
fn punctuation_key_info(ch: char) -> (&'static str, i32) {
    match ch {
        // VK_OEM_1 (0xBA = 186) — ";:" key on US layout
        ';' | ':' => ("Semicolon", 186),
        // VK_OEM_PLUS (0xBB = 187) — "=+" key
        '=' | '+' => ("Equal", 187),
        // VK_OEM_COMMA (0xBC = 188) — ",<" key
        ',' | '<' => ("Comma", 188),
        // VK_OEM_MINUS (0xBD = 189) — "-_" key
        '-' | '_' => ("Minus", 189),
        // VK_OEM_PERIOD (0xBE = 190) — ".>" key
        '.' | '>' => ("Period", 190),
        // VK_OEM_2 (0xBF = 191) — "/?" key
        '/' | '?' => ("Slash", 191),
        // VK_OEM_3 (0xC0 = 192) — "`~" key
        '`' | '~' => ("Backquote", 192),
        // VK_OEM_4 (0xDB = 219) — "[{" key
        '[' | '{' => ("BracketLeft", 219),
        // VK_OEM_5 (0xDC = 220) — "\\|" key
        '\\' | '|' => ("Backslash", 220),
        // VK_OEM_6 (0xDD = 221) — "]}" key
        ']' | '}' => ("BracketRight", 221),
        // VK_OEM_7 (0xDE = 222) — "'\""" key
        '\'' | '"' => ("Quote", 222),
        _ => ("", 0),
    }
}

/// Return the `text` value that CDP `Input.dispatchKeyEvent` needs on the
/// `keyDown` event so that Chrome performs the default action for the key.
/// For example Enter needs `"\r"` to actually submit a form, and Tab needs
/// `"\t"` to move focus.  Non-printable / navigation keys return `None`.
fn key_text(key_name: &str) -> Option<String> {
    match key_name {
        "Enter" => Some("\r".to_string()),
        "Tab" => Some("\t".to_string()),
        " " => Some(" ".to_string()),
        _ => {
            // Single printable characters carry themselves as text.
            if key_name.len() == 1 {
                Some(key_name.to_string())
            } else {
                None
            }
        }
    }
}

fn named_key_info(key: &str) -> (String, String, i32) {
    match key.to_lowercase().as_str() {
        "enter" | "return" => ("Enter".to_string(), "Enter".to_string(), 13),
        "tab" => ("Tab".to_string(), "Tab".to_string(), 9),
        "escape" | "esc" => ("Escape".to_string(), "Escape".to_string(), 27),
        "backspace" => ("Backspace".to_string(), "Backspace".to_string(), 8),
        "delete" => ("Delete".to_string(), "Delete".to_string(), 46),
        "arrowup" | "up" => ("ArrowUp".to_string(), "ArrowUp".to_string(), 38),
        "arrowdown" | "down" => ("ArrowDown".to_string(), "ArrowDown".to_string(), 40),
        "arrowleft" | "left" => ("ArrowLeft".to_string(), "ArrowLeft".to_string(), 37),
        "arrowright" | "right" => ("ArrowRight".to_string(), "ArrowRight".to_string(), 39),
        "home" => ("Home".to_string(), "Home".to_string(), 36),
        "end" => ("End".to_string(), "End".to_string(), 35),
        "pageup" => ("PageUp".to_string(), "PageUp".to_string(), 33),
        "pagedown" => ("PageDown".to_string(), "PageDown".to_string(), 34),
        "space" | " " => (" ".to_string(), "Space".to_string(), 32),
        _ => {
            if key.len() == 1 {
                let ch = key.chars().next().unwrap();
                char_to_key_info(ch)
            } else {
                (key.to_string(), key.to_string(), 0)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verify that `char_to_key_info` returns the correct (key, code,
    /// windowsVirtualKeyCode) triple for every character in Playwright's
    /// USKeyboardLayout.  The expected values below are taken verbatim from
    /// playwright-core/lib/server/usKeyboardLayout.js so that any drift from
    /// Playwright's behaviour is caught immediately.
    #[test]
    fn test_char_to_key_info_matches_playwright_layout() {
        // (character, expected_code, expected_vk_code)
        let cases: &[(char, &str, i32)] = &[
            // Letters – VK code must equal the uppercase ASCII value.
            ('a', "KeyA", 65),
            ('z', "KeyZ", 90),
            ('A', "KeyA", 65),
            // Digits
            ('0', "Digit0", 48),
            ('9', "Digit9", 57),
            // Punctuation – these are the values from Playwright's layout.
            // The bug that prompted this test sent '.' as VK 46 (= VK_DELETE).
            ('.', "Period", 190),
            (',', "Comma", 188),
            ('/', "Slash", 191),
            (';', "Semicolon", 186),
            ('\'', "Quote", 222),
            ('[', "BracketLeft", 219),
            (']', "BracketRight", 221),
            ('\\', "Backslash", 220),
            ('`', "Backquote", 192),
            ('-', "Minus", 189),
            ('=', "Equal", 187),
            // Shifted variants produced by the same physical keys.
            ('>', "Period", 190),
            ('<', "Comma", 188),
            ('?', "Slash", 191),
            (':', "Semicolon", 186),
            ('"', "Quote", 222),
            ('{', "BracketLeft", 219),
            ('}', "BracketRight", 221),
            ('|', "Backslash", 220),
            ('~', "Backquote", 192),
            ('_', "Minus", 189),
            ('+', "Equal", 187),
            // Whitespace / control
            (' ', "Space", 32),
            ('\n', "Enter", 13),
            ('\t', "Tab", 9),
        ];

        for &(ch, expected_code, expected_vk) in cases {
            let (key, code, vk) = char_to_key_info(ch);
            assert_eq!(
                code, expected_code,
                "char {:?}: expected code {:?}, got {:?}",
                ch, expected_code, code
            );
            assert_eq!(
                vk, expected_vk,
                "char {:?}: expected VK {}, got {} (ASCII would be {})",
                ch, expected_vk, vk, ch as i32
            );
            // key should be the character itself (except control chars).
            if !ch.is_control() {
                assert_eq!(key, ch.to_string(), "char {:?}: key mismatch", ch);
            }
        }
    }

    /// Regression test: period must NEVER map to VK 46 (VK_DELETE).
    #[test]
    fn test_period_is_not_vk_delete() {
        let (_, _, vk) = char_to_key_info('.');
        assert_ne!(
            vk, 46,
            "Period must not use VK code 46 (VK_DELETE); expected 190 (VK_OEM_PERIOD)"
        );
        assert_eq!(vk, 190);
    }

    /// Characters outside the US keyboard layout should return (key, "", 0)
    /// so that `type_text` falls back to `Input.insertText`.
    #[test]
    fn test_unmapped_chars_return_zero_keycode() {
        for ch in ['@', '#', '$', '%', '^', '&', '*', '(', ')', '€', '£', '你'] {
            let (key, code, vk) = char_to_key_info(ch);
            assert_eq!(
                code, "",
                "char {:?}: unmapped char should have empty code, got {:?}",
                ch, code
            );
            assert_eq!(
                vk, 0,
                "char {:?}: unmapped char should have VK 0, got {}",
                ch, vk
            );
            assert_eq!(key, ch.to_string());
        }
    }

    #[test]
    fn test_key_text_returns_correct_text_for_special_keys() {
        assert_eq!(key_text("Enter"), Some("\r".to_string()));
        assert_eq!(key_text("Tab"), Some("\t".to_string()));
        assert_eq!(key_text(" "), Some(" ".to_string()));
        // Single printable characters carry themselves.
        assert_eq!(key_text("a"), Some("a".to_string()));
        assert_eq!(key_text("Z"), Some("Z".to_string()));
        // Non-printable named keys return None.
        assert_eq!(key_text("Escape"), None);
        assert_eq!(key_text("ArrowUp"), None);
        assert_eq!(key_text("Backspace"), None);
        assert_eq!(key_text("Delete"), None);
    }
}
#[allow(dead_code, unused_imports)]
pub(crate) mod action_commands {
    use crate::native::action_runtime::runtime::{
        is_stale_page_session_error, optional_command_string, recover_browser_command_channel,
        relaunch_and_restore_page, service_browser_id,
        validate_service_tab_handle_for_current_session,
        validate_service_tab_handle_route_for_current_session, DaemonState, FetchPausedRequest,
        HarEntry, MouseState, RouteEntry, RouteResponse, TrackedRequest,
        AUTH_LOGIN_PREFERRED_SELECTOR_WINDOW_MS, AUTH_LOGIN_SELECTOR_POLL_INTERVAL_MS,
        AUTH_LOGIN_WAIT_UNTIL,
    };
    use crate::native::browser::{
        should_track_target, BrowserManager, BrowserShutdownOutcome, PageInfo,
        ProcessExitObservation, WaitUntil,
    };
    use crate::native::browser_wait::{
        wait_for_function, wait_for_selector, wait_for_text, wait_for_url,
    };
    use crate::native::cdp::client::CdpClient;
    use crate::native::cdp::types::{
        AttachToTargetParams, AttachToTargetResult, CdpEvent, CreateTargetResult,
        DispatchMouseEventParams, ExceptionThrownEvent, JavascriptDialogOpeningEvent,
        TargetCreatedEvent, TargetDestroyedEvent, TargetInfoChangedEvent,
    };
    use crate::native::element::RefMap;
    use crate::native::interaction;
    use crate::native::service_diagnostics::truncate_utf8;
    use crate::native::state;
    use crate::native::webdriver::backend::BrowserBackend;
    use serde_json::{json, Map, Value};
    use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
    use std::time::{Duration, Instant};
    pub(crate) async fn handle_click(
        cmd: &Value,
        state: &mut DaemonState,
    ) -> Result<Value, String> {
        let capture_clipboard_write = cmd
            .get("captureClipboardWrite")
            .and_then(|value| value.as_bool())
            .unwrap_or(false);
        if !capture_clipboard_write {
            return handle_click_action(cmd, state).await;
        }
        let (client, session_id) = {
            let mgr = state
                .browser
                .as_ref()
                .ok_or("--capture-clipboard-write requires the native Chrome browser backend")?;
            (mgr.client.clone(), mgr.active_session_id()?.to_string())
        };
        let action = handle_click_action(cmd, state);
        let action_timeout = cmd
            .get("jobTimeoutMs")
            .and_then(|value| value.as_u64())
            .map(|timeout_ms| {
                tokio::time::Duration::from_millis(timeout_ms.saturating_sub(1000).max(1))
            })
            .unwrap_or(super::super::clipboard::DEFAULT_WRITE_CAPTURE_ACTION_TIMEOUT);
        let (action_result, capture) = super::super::clipboard::capture_write_during(
            &client,
            &session_id,
            super::super::clipboard::DEFAULT_WRITE_CAPTURE_LIMIT,
            action_timeout,
            action,
        )
        .await?;
        match action_result {
            Ok(mut response) => {
                response["clipboardCapture"] = json!(
                    { "supported" : capture.supported, "invoked" : capture.invoked,
                    "text" : capture.text, "truncated" : capture.truncated,
                    "originalLength" : capture.original_length, "restored" : capture
                    .restored, "reason" : capture.reason, }
                );
                Ok(response)
            }
            Err(error) => Err(format!(
                "{error}; clipboardCaptureRestored={}",
                capture.restored
            )),
        }
    }
    pub(crate) async fn handle_click_action(
        cmd: &Value,
        state: &mut DaemonState,
    ) -> Result<Value, String> {
        let selector = cmd
            .get("selector")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'selector' parameter")?;
        if let Some(ref wb) = state.webdriver_backend {
            if state.browser.is_none() {
                wb.click(selector).await?;
                return Ok(json!({ "clicked" : selector }));
            }
        }
        let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
        let client = mgr.client.clone();
        let session_id = mgr.active_session_id()?.to_string();
        let new_tab = cmd.get("newTab").and_then(|v| v.as_bool()).unwrap_or(false);
        if new_tab {
            use super::super::element::resolve_element_object_id;
            let (object_id, effective_session_id) = resolve_element_object_id(
                &client,
                &session_id,
                &state.ref_map,
                selector,
                &state.iframe_sessions,
            )
            .await?;
            let call_params = json!(
                { "objectId" : object_id, "functionDeclaration" :
                "function() { var h = this.getAttribute('href'); if (!h) return null; try { return new URL(h, document.baseURI).toString(); } catch(e) { return null; } }",
                "returnByValue" : true }
            );
            let call_result = client
                .send_command(
                    "Runtime.callFunctionOn",
                    Some(call_params),
                    Some(&effective_session_id),
                )
                .await?;
            let href = call_result
                .get("result")
                .and_then(|r| r.get("value"))
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    format!(
                        "Element '{}' does not have an href attribute. --new-tab only works on links.",
                        selector
                    )
                })?
                .to_string();
            let mgr = state.browser.as_mut().ok_or("Browser not launched")?;
            state.ref_map.clear();
            mgr.tab_new(Some(&href)).await?;
            return Ok(json!({ "clicked" : selector, "newTab" : true, "url" : href }));
        }
        let button = cmd.get("button").and_then(|v| v.as_str()).unwrap_or("left");
        let click_count = cmd.get("clickCount").and_then(|v| v.as_i64()).unwrap_or(1) as i32;
        if button == "left" && click_count == 1 {
            if let Some(ref_id) = super::super::element::parse_ref(selector) {
                if let Some(entry) = state.ref_map.get(&ref_id) {
                    if entry.role == "link" {
                        let nth = entry.nth.unwrap_or(0);
                        let link_lookup: Value = client
                            .send_command_typed(
                                "Runtime.evaluate",
                                &super::super::cdp::types::EvaluateParams {
                                    expression: format!(
                                        r#"(function() {{
                                        const targetName = {name};
                                        const targetIndex = {nth};
                                        const links = Array.from(document.querySelectorAll('a[href]'))
                                            .filter((el) => {{
                                                const rect = el.getBoundingClientRect();
                                                const style = window.getComputedStyle(el);
                                                if (rect.width <= 0 || rect.height <= 0) return false;
                                                if (style.display === 'none' || style.visibility === 'hidden' || Number.parseFloat(style.opacity || '1') === 0) return false;
                                                const label = (el.getAttribute('aria-label') || el.getAttribute('title') || el.innerText || el.textContent || '').trim().replace(/\s+/g, ' ');
                                                return label === targetName;
                                            }});
                                        const el = links[targetIndex];
                                        return el ? el.href : null;
                                    }})()"#,
                                        name = serde_json::to_string(& entry.name).unwrap_or_else(|
                                        _ | "\"\"".to_string()), nth = nth,
                                    ),
                                    return_by_value: Some(true),
                                    await_promise: Some(true),
                                },
                                Some(&session_id),
                            )
                            .await
                            .ok()
                            .and_then(|r: super::super::cdp::types::EvaluateResult| {
                                r.result.value
                            })
                            .unwrap_or(Value::Null);
                        if let Some(href) = link_lookup.as_str() {
                            if let Some(mgr) = state.browser.as_mut() {
                                mgr.set_active_page_url(href);
                            }
                            let _ = client
                                .send_command_typed::<_, super::super::cdp::types::EvaluateResult>(
                                    "Runtime.evaluate",
                                    &super::super::cdp::types::EvaluateParams {
                                        expression: format!(
                                            "window.location.assign({});",
                                            serde_json::to_string(href)
                                                .unwrap_or_else(|_| "\"\"".to_string())
                                        ),
                                        return_by_value: Some(true),
                                        await_promise: Some(false),
                                    },
                                    Some(&session_id),
                                )
                                .await;
                            return Ok(json!(
                                { "clicked" : selector, "url" : href, "fallbackNavigation" :
                                true }
                            ));
                        }
                    }
                }
            }
            use super::super::element::resolve_element_object_id;
            let (object_id, effective_session_id) = resolve_element_object_id(
                &client,
                &session_id,
                &state.ref_map,
                selector,
                &state.iframe_sessions,
            )
            .await?;
            let call_result = client
                .send_command(
                    "Runtime.callFunctionOn",
                    Some(json!(
                        { "objectId" : object_id, "functionDeclaration" :
                        r#"function() {
                        const el = this.closest?.('a[href]') || this;
                        if (!el || !el.href) return null;
                        const href = String(el.href);
                        const target = el.getAttribute('target') || '';
                        if (target && target !== '_self') return null;
                        if (href.startsWith('javascript:')) return null;
                        return href;
                    }"#,
                        "returnByValue" : true }
                    )),
                    Some(&effective_session_id),
                )
                .await?;
            if let Some(href) = call_result
                .get("result")
                .and_then(|r| r.get("value"))
                .and_then(|v| v.as_str())
            {
                interaction::focus(
                    &client,
                    &session_id,
                    &state.ref_map,
                    selector,
                    &state.iframe_sessions,
                )
                .await?;
                let press_client = client.clone();
                let press_session_id = session_id.clone();
                tokio::spawn(async move {
                    tokio::time::sleep(tokio::time::Duration::from_millis(75)).await;
                    let _ = interaction::press_key(&press_client, &press_session_id, "Enter").await;
                });
                return Ok(json!(
                    { "clicked" : selector, "url" : href, "deferredActivation" : true
                    }
                ));
            }
        }
        interaction::click(
            &client,
            &session_id,
            &state.ref_map,
            selector,
            button,
            click_count,
            &state.iframe_sessions,
        )
        .await?;
        Ok(json!({ "clicked" : selector }))
    }
    pub(crate) async fn handle_dblclick(
        cmd: &Value,
        state: &mut DaemonState,
    ) -> Result<Value, String> {
        let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
        let session_id = mgr.active_session_id()?.to_string();
        let selector = cmd
            .get("selector")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'selector' parameter")?;
        interaction::dblclick(
            &mgr.client,
            &session_id,
            &state.ref_map,
            selector,
            &state.iframe_sessions,
        )
        .await?;
        Ok(json!({ "clicked" : selector }))
    }
    pub(crate) async fn handle_fill(cmd: &Value, state: &mut DaemonState) -> Result<Value, String> {
        let selector = cmd
            .get("selector")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'selector' parameter")?;
        let value = cmd
            .get("value")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'value' parameter")?;
        if let Some(ref wb) = state.webdriver_backend {
            if state.browser.is_none() {
                wb.fill(selector, value).await?;
                return Ok(json!({ "filled" : selector }));
            }
        }
        let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
        let session_id = mgr.active_session_id()?.to_string();
        interaction::fill(
            &mgr.client,
            &session_id,
            &state.ref_map,
            selector,
            value,
            &state.iframe_sessions,
        )
        .await?;
        Ok(json!({ "filled" : selector }))
    }
    pub(crate) async fn handle_type(cmd: &Value, state: &mut DaemonState) -> Result<Value, String> {
        let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
        let session_id = mgr.active_session_id()?.to_string();
        let selector = cmd
            .get("selector")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'selector' parameter")?;
        let text = cmd
            .get("text")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'text' parameter")?;
        let clear = cmd.get("clear").and_then(|v| v.as_bool()).unwrap_or(false);
        let delay = cmd.get("delay").and_then(|v| v.as_u64());
        interaction::type_text(
            &mgr.client,
            &session_id,
            &state.ref_map,
            selector,
            text,
            clear,
            delay,
            &state.iframe_sessions,
        )
        .await?;
        Ok(json!({ "typed" : text }))
    }
    pub(crate) async fn handle_press(
        cmd: &Value,
        state: &mut DaemonState,
    ) -> Result<Value, String> {
        let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
        let session_id = mgr.active_session_id()?.to_string();
        let key = cmd
            .get("key")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'key' parameter")?;
        let (actual_key, modifiers) = parse_key_chord(key);
        interaction::press_key_with_modifiers(&mgr.client, &session_id, &actual_key, modifiers)
            .await?;
        Ok(json!({ "pressed" : key }))
    }
    /// Parse a key chord string like "Control+a" or "Control+Shift+Enter" into
    /// the actual key name and an optional CDP modifier bitmask.
    ///
    /// CDP modifier values: 1 = Alt, 2 = Control, 4 = Meta (Cmd), 8 = Shift.
    pub(crate) fn parse_key_chord(input: &str) -> (String, Option<i32>) {
        let parts: Vec<&str> = input.split('+').collect();
        if parts.len() < 2 {
            return (input.to_string(), None);
        }
        let mut modifiers = 0i32;
        let mut key_parts: Vec<&str> = Vec::new();
        for part in &parts {
            match part.to_lowercase().as_str() {
                "alt" => modifiers |= 1,
                "control" | "ctrl" => modifiers |= 2,
                "meta" | "cmd" | "command" => modifiers |= 4,
                "shift" => modifiers |= 8,
                _ => key_parts.push(part),
            }
        }
        if modifiers == 0 {
            return (input.to_string(), None);
        }
        let actual_key = if key_parts.is_empty() {
            input.to_string()
        } else {
            key_parts.join("+")
        };
        (actual_key, Some(modifiers))
    }
    pub(crate) async fn handle_hover(
        cmd: &Value,
        state: &mut DaemonState,
    ) -> Result<Value, String> {
        let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
        let session_id = mgr.active_session_id()?.to_string();
        let selector = cmd
            .get("selector")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'selector' parameter")?;
        interaction::hover(
            &mgr.client,
            &session_id,
            &state.ref_map,
            selector,
            &state.iframe_sessions,
        )
        .await?;
        Ok(json!({ "hovered" : selector }))
    }
    pub(crate) async fn handle_scroll(
        cmd: &Value,
        state: &mut DaemonState,
    ) -> Result<Value, String> {
        let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
        let session_id = mgr.active_session_id()?.to_string();
        let selector = cmd.get("selector").and_then(|v| v.as_str());
        let (mut dx, mut dy) = (
            cmd.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0),
            cmd.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0),
        );
        if let Some(direction) = cmd.get("direction").and_then(|v| v.as_str()) {
            let amount = cmd.get("amount").and_then(|v| v.as_f64()).unwrap_or(300.0);
            match direction {
                "up" => dy = -amount,
                "down" => dy = amount,
                "left" => dx = -amount,
                "right" => dx = amount,
                _ => {}
            }
        }
        interaction::scroll(
            &mgr.client,
            &session_id,
            &state.ref_map,
            selector,
            dx,
            dy,
            &state.iframe_sessions,
        )
        .await?;
        Ok(json!({ "scrolled" : true }))
    }
    pub(crate) async fn handle_select(
        cmd: &Value,
        state: &mut DaemonState,
    ) -> Result<Value, String> {
        let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
        let session_id = mgr.active_session_id()?.to_string();
        let selector = cmd
            .get("selector")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'selector' parameter")?;
        let values: Vec<String> = match cmd.get("values") {
            Some(Value::Array(arr)) => arr
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect(),
            Some(Value::String(s)) => vec![s.clone()],
            _ => cmd
                .get("value")
                .and_then(|v| v.as_str())
                .map(|s| vec![s.to_string()])
                .unwrap_or_default(),
        };
        interaction::select_option(
            &mgr.client,
            &session_id,
            &state.ref_map,
            selector,
            &values,
            &state.iframe_sessions,
        )
        .await?;
        Ok(json!({ "selected" : values }))
    }
    pub(crate) async fn handle_check(
        cmd: &Value,
        state: &mut DaemonState,
    ) -> Result<Value, String> {
        let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
        let session_id = mgr.active_session_id()?.to_string();
        let selector = cmd
            .get("selector")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'selector' parameter")?;
        interaction::check(
            &mgr.client,
            &session_id,
            &state.ref_map,
            selector,
            &state.iframe_sessions,
        )
        .await?;
        Ok(json!({ "checked" : selector }))
    }
    pub(crate) async fn handle_uncheck(
        cmd: &Value,
        state: &mut DaemonState,
    ) -> Result<Value, String> {
        let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
        let session_id = mgr.active_session_id()?.to_string();
        let selector = cmd
            .get("selector")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'selector' parameter")?;
        interaction::uncheck(
            &mgr.client,
            &session_id,
            &state.ref_map,
            selector,
            &state.iframe_sessions,
        )
        .await?;
        Ok(json!({ "unchecked" : selector }))
    }
    pub(crate) async fn handle_wait(cmd: &Value, state: &mut DaemonState) -> Result<Value, String> {
        let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
        let session_id = mgr.active_session_id()?.to_string();
        let timeout_ms = state.timeout_ms(cmd);
        if let Some(text) = cmd.get("text").and_then(|v| v.as_str()) {
            wait_for_text(&mgr.client, &session_id, text, timeout_ms).await?;
            return Ok(json!({ "waited" : "text", "text" : text }));
        }
        if let Some(selector) = cmd.get("selector").and_then(|v| v.as_str()) {
            let state_str = cmd
                .get("state")
                .and_then(|v| v.as_str())
                .unwrap_or("visible");
            wait_for_selector(&mgr.client, &session_id, selector, state_str, timeout_ms).await?;
            return Ok(json!({ "waited" : "selector", "selector" : selector }));
        }
        if let Some(url_pattern) = cmd.get("url").and_then(|v| v.as_str()) {
            wait_for_url(&mgr.client, &session_id, url_pattern, timeout_ms).await?;
            return Ok(json!({ "waited" : "url", "url" : url_pattern }));
        }
        if let Some(fn_str) = cmd.get("function").and_then(|v| v.as_str()) {
            wait_for_function(&mgr.client, &session_id, fn_str, timeout_ms).await?;
            return Ok(json!({ "waited" : "function" }));
        }
        if let Some(load_state) = cmd.get("loadState").and_then(|v| v.as_str()) {
            let wait_until = WaitUntil::from_str(load_state);
            mgr.wait_for_lifecycle_external(wait_until, &session_id)
                .await?;
            return Ok(json!({ "waited" : "load", "state" : load_state }));
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(timeout_ms)).await;
        Ok(json!({ "waited" : "timeout", "ms" : timeout_ms }))
    }
    pub(crate) async fn handle_gettext(
        cmd: &Value,
        state: &mut DaemonState,
    ) -> Result<Value, String> {
        let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
        let session_id = mgr.active_session_id()?.to_string();
        let selector = cmd
            .get("selector")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'selector' parameter")?;
        let text = super::super::element::get_element_text(
            &mgr.client,
            &session_id,
            &state.ref_map,
            selector,
            &state.iframe_sessions,
        )
        .await?;
        let url = mgr.get_url().await.unwrap_or_default();
        Ok(json!({ "text" : text, "origin" : url }))
    }
    pub(crate) async fn handle_getattribute(
        cmd: &Value,
        state: &mut DaemonState,
    ) -> Result<Value, String> {
        let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
        let session_id = mgr.active_session_id()?.to_string();
        let selector = cmd
            .get("selector")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'selector' parameter")?;
        let attribute = cmd
            .get("attribute")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'attribute' parameter")?;
        let value = super::super::element::get_element_attribute(
            &mgr.client,
            &session_id,
            &state.ref_map,
            selector,
            attribute,
            &state.iframe_sessions,
        )
        .await?;
        let url = mgr.get_url().await.unwrap_or_default();
        Ok(json!({ "value" : value, "origin" : url }))
    }
    pub(crate) async fn handle_isvisible(
        cmd: &Value,
        state: &mut DaemonState,
    ) -> Result<Value, String> {
        let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
        let session_id = mgr.active_session_id()?.to_string();
        let selector = cmd
            .get("selector")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'selector' parameter")?;
        let visible = super::super::element::is_element_visible(
            &mgr.client,
            &session_id,
            &state.ref_map,
            selector,
            &state.iframe_sessions,
        )
        .await?;
        let url = mgr.get_url().await.unwrap_or_default();
        Ok(json!({ "visible" : visible, "origin" : url }))
    }
    pub(crate) async fn handle_isenabled(
        cmd: &Value,
        state: &mut DaemonState,
    ) -> Result<Value, String> {
        let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
        let session_id = mgr.active_session_id()?.to_string();
        let selector = cmd
            .get("selector")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'selector' parameter")?;
        let enabled = super::super::element::is_element_enabled(
            &mgr.client,
            &session_id,
            &state.ref_map,
            selector,
            &state.iframe_sessions,
        )
        .await?;
        let url = mgr.get_url().await.unwrap_or_default();
        Ok(json!({ "enabled" : enabled, "origin" : url }))
    }
    pub(crate) async fn handle_ischecked(
        cmd: &Value,
        state: &mut DaemonState,
    ) -> Result<Value, String> {
        let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
        let session_id = mgr.active_session_id()?.to_string();
        let selector = cmd
            .get("selector")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'selector' parameter")?;
        let checked = super::super::element::is_element_checked(
            &mgr.client,
            &session_id,
            &state.ref_map,
            selector,
            &state.iframe_sessions,
        )
        .await?;
        let url = mgr.get_url().await.unwrap_or_default();
        Ok(json!({ "checked" : checked, "origin" : url }))
    }
    pub(crate) async fn handle_focus(
        cmd: &Value,
        state: &mut DaemonState,
    ) -> Result<Value, String> {
        let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
        let session_id = mgr.active_session_id()?.to_string();
        let selector = cmd
            .get("selector")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'selector' parameter")?;
        interaction::focus(
            &mgr.client,
            &session_id,
            &state.ref_map,
            selector,
            &state.iframe_sessions,
        )
        .await?;
        Ok(json!({ "focused" : selector }))
    }
    pub(crate) async fn handle_clear(
        cmd: &Value,
        state: &mut DaemonState,
    ) -> Result<Value, String> {
        let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
        let session_id = mgr.active_session_id()?.to_string();
        let selector = cmd
            .get("selector")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'selector' parameter")?;
        interaction::clear(
            &mgr.client,
            &session_id,
            &state.ref_map,
            selector,
            &state.iframe_sessions,
        )
        .await?;
        Ok(json!({ "cleared" : selector }))
    }
    pub(crate) async fn handle_selectall(
        cmd: &Value,
        state: &mut DaemonState,
    ) -> Result<Value, String> {
        let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
        let session_id = mgr.active_session_id()?.to_string();
        let selector = cmd
            .get("selector")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'selector' parameter")?;
        interaction::select_all(
            &mgr.client,
            &session_id,
            &state.ref_map,
            selector,
            &state.iframe_sessions,
        )
        .await?;
        Ok(json!({ "selected" : selector }))
    }
    pub(crate) async fn handle_scrollintoview(
        cmd: &Value,
        state: &mut DaemonState,
    ) -> Result<Value, String> {
        let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
        let session_id = mgr.active_session_id()?.to_string();
        let selector = cmd
            .get("selector")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'selector' parameter")?;
        interaction::scroll_into_view(
            &mgr.client,
            &session_id,
            &state.ref_map,
            selector,
            &state.iframe_sessions,
        )
        .await?;
        Ok(json!({ "scrolled" : selector }))
    }
    pub(crate) async fn handle_dispatch(
        cmd: &Value,
        state: &mut DaemonState,
    ) -> Result<Value, String> {
        let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
        let session_id = mgr.active_session_id()?.to_string();
        let selector = cmd
            .get("selector")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'selector' parameter")?;
        let event_type = cmd
            .get("event")
            .or_else(|| cmd.get("eventType"))
            .and_then(|v| v.as_str())
            .ok_or("Missing 'event' parameter")?;
        let event_init = cmd.get("eventInit");
        interaction::dispatch_event(
            &mgr.client,
            &session_id,
            &state.ref_map,
            selector,
            event_type,
            event_init,
            &state.iframe_sessions,
        )
        .await?;
        Ok(json!({ "dispatched" : event_type, "selector" : selector }))
    }
    pub(crate) async fn handle_highlight(
        cmd: &Value,
        state: &mut DaemonState,
    ) -> Result<Value, String> {
        let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
        let session_id = mgr.active_session_id()?.to_string();
        let selector = cmd
            .get("selector")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'selector' parameter")?;
        interaction::highlight(
            &mgr.client,
            &session_id,
            &state.ref_map,
            selector,
            &state.iframe_sessions,
        )
        .await?;
        Ok(json!({ "highlighted" : selector }))
    }
    pub(crate) async fn handle_tap(cmd: &Value, state: &mut DaemonState) -> Result<Value, String> {
        let selector = cmd.get("selector").and_then(|v| v.as_str());
        if let Some(ref appium) = state.appium {
            if state.browser.is_none() {
                let x = cmd.get("x").and_then(|v| v.as_f64()).unwrap_or(200.0);
                let y = cmd.get("y").and_then(|v| v.as_f64()).unwrap_or(200.0);
                appium.tap(x, y).await?;
                return Ok(json!({ "tapped" : true, "x" : x, "y" : y }));
            }
        }
        let sel = selector.ok_or("Missing 'selector' parameter")?;
        let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
        let session_id = mgr.active_session_id()?.to_string();
        interaction::tap_touch(
            &mgr.client,
            &session_id,
            &state.ref_map,
            sel,
            &state.iframe_sessions,
        )
        .await?;
        Ok(json!({ "tapped" : sel }))
    }
    pub(crate) async fn handle_dialog(
        cmd: &Value,
        state: &mut DaemonState,
    ) -> Result<Value, String> {
        let response = cmd.get("response").and_then(|v| v.as_str());
        if response == Some("status") {
            return Ok(match &state.pending_dialog {
                Some(dialog) => {
                    let mut obj = json!(
                        { "hasDialog" : true, "type" : dialog.dialog_type, "message"
                        : dialog.message, }
                    );
                    if let Some(ref prompt) = dialog.default_prompt {
                        obj["defaultPrompt"] = json!(prompt);
                    }
                    obj
                }
                None => json!({ "hasDialog" : false }),
            });
        }
        let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
        let accept = response
            .map(|r| r == "accept")
            .or_else(|| cmd.get("accept").and_then(|v| v.as_bool()))
            .unwrap_or(true);
        let prompt_text = cmd.get("promptText").and_then(|v| v.as_str());
        mgr.handle_dialog(accept, prompt_text).await?;
        state.pending_dialog = None;
        Ok(json!({ "handled" : true, "accepted" : accept }))
    }
    pub(crate) async fn handle_upload(cmd: &Value, state: &DaemonState) -> Result<Value, String> {
        let mgr = state.browser.as_ref().ok_or("Browser not launched")?;
        let selector = cmd
            .get("selector")
            .and_then(|v| v.as_str())
            .ok_or("Missing 'selector' parameter")?;
        let files: Vec<String> = cmd
            .get("files")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .or_else(|| {
                cmd.get("file")
                    .and_then(|v| v.as_str())
                    .map(|s| vec![s.to_string()])
            })
            .unwrap_or_default();
        let session_id = mgr.active_session_id()?.to_string();
        let (object_id, effective_session_id) = super::super::element::resolve_element_object_id(
            &mgr.client,
            &session_id,
            &state.ref_map,
            selector,
            &state.iframe_sessions,
        )
        .await?;
        mgr.client
            .send_command(
                "DOM.setFileInputFiles",
                Some(json!({ "files" : files, "objectId" : object_id, })),
                Some(&effective_session_id),
            )
            .await?;
        Ok(json!({ "uploaded" : files.len(), "selector" : selector }))
    }
}
pub(crate) use action_commands::*;
#[cfg(test)]
mod action_tests;
