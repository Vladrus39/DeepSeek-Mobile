use deepseek_mobile_core::{ApprovalCardSeverity, ApprovalCardView, ReviewDecision};
use dioxus::prelude::*;

pub fn mobile_approval_panel(
    cards: &[ApprovalCardView],
    on_decision: EventHandler<(String, ReviewDecision)>,
) -> Element {
    if cards.is_empty() {
        return rsx! {};
    }

    rsx! {
        div {
            display: "flex",
            flex_direction: "column",
            gap: "10px",
            margin_bottom: "10px",

            for card in cards.iter() {
                div {
                    key: "{card.id}",
                    background_color: severity_background(&card.severity),
                    border: severity_border(&card.severity),
                    border_radius: "16px",
                    padding: "12px",
                    display: "flex",
                    flex_direction: "column",
                    gap: "8px",

                    div {
                        display: "flex",
                        justify_content: "space-between",
                        align_items: "center",
                        gap: "8px",

                        div {
                            font_size: "14px",
                            font_weight: "700",
                            color: "white",
                            "{card.title}"
                        }

                        div {
                            color: severity_text(&card.severity),
                            font_size: "10px",
                            font_weight: "700",
                            text_transform: "uppercase",
                            "{card.tool_name}"
                        }
                    }

                    div {
                        color: "#d1d5db",
                        font_size: "12px",
                        "{card.subtitle}"
                    }

                    div {
                        color: "#9ca3af",
                        font_size: "12px",
                        "{card.description}"
                    }

                    if !card.impacts.is_empty() {
                        div {
                            display: "flex",
                            flex_direction: "column",
                            gap: "4px",
                            for impact in card.impacts.iter() {
                                div {
                                    color: "#d1d5db",
                                    font_size: "11px",
                                    "• {impact}"
                                }
                            }
                        }
                    }

                    div {
                        background_color: "rgba(17, 24, 39, 0.7)",
                        border: "1px solid rgba(75, 85, 99, 0.7)",
                        border_radius: "12px",
                        padding: "8px",
                        color: "#e5e7eb",
                        font_size: "11px",
                        white_space: "pre-wrap",
                        "{format_argument_preview(card)}"
                    }

                    div {
                        display: "flex",
                        flex_wrap: "wrap",
                        gap: "8px",

                        for action in card.actions.iter() {
                            {
                                let approval_id = card.id.clone();
                                let decision = action.decision.clone();
                                rsx! {
                                    button {
                                        background_color: action_background(&action.decision),
                                        color: "white",
                                        border: "none",
                                        border_radius: "999px",
                                        padding: "8px 12px",
                                        font_size: "12px",
                                        font_weight: "700",
                                        onclick: move |_| on_decision.call((approval_id.clone(), decision.clone())),
                                        "{action.label}"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn format_argument_preview(card: &ApprovalCardView) -> String {
    card.argument_preview.to_string()
}

fn severity_background(severity: &ApprovalCardSeverity) -> &'static str {
    match severity {
        ApprovalCardSeverity::Info => "#172554",
        ApprovalCardSeverity::Warning => "#422006",
        ApprovalCardSeverity::HighRisk => "#450a0a",
        ApprovalCardSeverity::Dangerous => "#7f1d1d",
    }
}

fn severity_border(severity: &ApprovalCardSeverity) -> &'static str {
    match severity {
        ApprovalCardSeverity::Info => "1px solid #2563eb",
        ApprovalCardSeverity::Warning => "1px solid #d97706",
        ApprovalCardSeverity::HighRisk => "1px solid #dc2626",
        ApprovalCardSeverity::Dangerous => "1px solid #ef4444",
    }
}

fn severity_text(severity: &ApprovalCardSeverity) -> &'static str {
    match severity {
        ApprovalCardSeverity::Info => "#93c5fd",
        ApprovalCardSeverity::Warning => "#fbbf24",
        ApprovalCardSeverity::HighRisk => "#fca5a5",
        ApprovalCardSeverity::Dangerous => "#fecaca",
    }
}

fn action_background(decision: &ReviewDecision) -> &'static str {
    match decision {
        ReviewDecision::Approved => "#2563eb",
        ReviewDecision::ApprovedForSession => "#059669",
        ReviewDecision::Denied => "#b45309",
        ReviewDecision::Abort => "#dc2626",
    }
}

#[cfg(test)]
mod tests {
    use super::{action_background, severity_border};
    use deepseek_mobile_core::{ApprovalCardSeverity, ReviewDecision};

    #[test]
    fn dangerous_cards_use_red_border() {
        assert!(severity_border(&ApprovalCardSeverity::Dangerous).contains("#ef4444"));
    }

    #[test]
    fn approve_for_session_uses_distinct_action_style() {
        assert_ne!(
            action_background(&ReviewDecision::Approved),
            action_background(&ReviewDecision::ApprovedForSession)
        );
    }
}