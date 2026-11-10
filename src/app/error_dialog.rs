//! Connection Error dialog (`ErrorDialog.tsx`): the message mapping and the persistent modal.
use gpui_kit::assets::IconName;
use gpui_kit::component::Icon;
use gpui_kit::*;

use crate::ui::button::{Button, Variant};
use crate::ui::modal::{ModalSize, OnClose, modal};
use crate::ui::theme::{Tw, hex, hexa, pal, tw};

/// Persistent `modal` (md); Escape and backdrop clicks do nothing, only the buttons (and the header X) close it.
pub fn connection_error(raw: &str, on_dismiss: OnClose, on_retry: OnClose, window: &mut Window, cx: &mut App) -> AnyElement {
    let (message, details) = describe_connection_error(raw);
    let p = pal(cx);
    let (dismiss, retry) = (on_dismiss.clone(), on_retry);
    let body = div()
        .p(px(24.))
        .child(
            div()
                .flex()
                .items_start()
                .gap(px(16.))
                .child(
                    div()
                        .flex_none()
                        .p(px(12.))
                        .rounded_full()
                        .bg(hexa(tw::RED_500, 0.1))
                        .child(Icon::new(IconName::TriangleAlert).size_6().text_color(hex(tw::RED_500))),
                )
                .child(
                    // `<p class="mb-2">` then `mt-3` box: block margins collapse to 12 px.
                    div()
                        .flex_1()
                        .min_w_0()
                        .child(
                            div()
                                .id("err-message")
                                .test_support()
                                .aria_label(message.clone())
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(p.text1)
                                .child(message),
                        )
                        .child(
                            div()
                                .id("err-details")
                                .test_support()
                                .aria_label(details.clone())
                                .mt(px(12.))
                                .p(px(12.))
                                .rounded(px(6.))
                                .bg(p.bg3)
                                .border_1()
                                .border_color(p.border)
                                .t_xs()
                                .mono()
                                .text_color(p.muted)
                                // ponytail: no CSS `break-all` in gpui; a long unbroken token wraps only at its own width
                                .child(details),
                        ),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .justify_end()
                .gap(px(12.))
                .mt(px(24.))
                .pt(px(16.))
                .border_t_1()
                .border_color(p.border)
                .child(
                    Button::new("err-dismiss", Variant::Secondary)
                        .icon(IconName::X)
                        .label("Dismiss")
                        .on_click(move |_, w, cx| dismiss(w, cx)),
                )
                .child(
                    Button::new("err-retry", Variant::Primary)
                        .icon(IconName::RefreshCw)
                        .label("Retry")
                        .on_click(move |_, w, cx| retry(w, cx)),
                ),
        )
        .into_any_element();
    modal("conn-error", "Connection Error", ModalSize::Md, true, on_dismiss, body, window, cx)
}

/// Maps a raw backend error to (message, details), checked in the React order.
pub fn describe_connection_error(raw: &str) -> (String, String) {
    let has = |s: &str| raw.contains(s);
    let (msg, details) = if has("No kubeconfig found") {
        (
            "Kubernetes configuration not found",
            "Please ensure kubectl is configured and ~/.kube/config exists.".to_string(),
        )
    } else if has("connection refused") || has("ECONNREFUSED") {
        (
            "Cannot reach Kubernetes cluster",
            "The cluster may be offline or unreachable. Check your network connection and cluster status."
                .to_string(),
        )
    } else if has("Unauthorized") || has("401") {
        (
            "Authentication failed",
            "Your Kubernetes credentials may have expired. Try running: kubectl get pods".to_string(),
        )
    } else if has("Forbidden") || has("403") {
        (
            "Access denied",
            "You do not have permission to access this resource. Check your RBAC settings.".to_string(),
        )
    } else if has("timeout") || has("ETIMEDOUT") || has("timed out") {
        (
            "Connection timed out",
            "The cluster took too long to respond. Check your network connection.".to_string(),
        )
    } else if has("certificate") {
        (
            "Certificate error",
            format!("There was a problem with the cluster certificate. {raw}"),
        )
    } else {
        ("Failed to connect to Kubernetes cluster", raw.to_string())
    };
    (msg.to_string(), details)
}

#[cfg(test)]
mod tests {
    use super::describe_connection_error;

    #[test]
    fn pinned_values() {
        assert_eq!(describe_connection_error("No kubeconfig found").0, "Kubernetes configuration not found");
        assert_eq!(describe_connection_error("Connection timed out after 30 seconds").0, "Connection timed out");
        assert_eq!(describe_connection_error("Kubernetes error: ApiError: Unauthorized (401)").0, "Authentication failed");
        assert_eq!(describe_connection_error("Kubernetes error: Forbidden").0, "Access denied");
        assert_eq!(describe_connection_error("weird").0, "Failed to connect to Kubernetes cluster");
        assert_eq!(describe_connection_error("weird").1, "weird");
    }
}
