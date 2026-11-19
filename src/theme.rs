use eframe::egui::{self, Color32, FontId, Stroke, TextStyle, Vec2};

pub const CANVAS: Color32 = Color32::WHITE;
pub const GRID_MINOR: Color32 = Color32::from_rgb(207, 238, 245);
pub const GRID_MAJOR: Color32 = Color32::from_rgb(156, 211, 225);
pub const FIELD_LINE: Color32 = Color32::from_rgb(137, 137, 137);
pub const YARD_NUMBER: Color32 = Color32::from_rgb(116, 116, 116);
pub const CHROME: Color32 = Color32::from_rgb(224, 224, 224);
pub const CHROME_LIGHT: Color32 = Color32::from_rgb(236, 236, 236);
pub const CHROME_DARK: Color32 = Color32::from_rgb(216, 216, 216);
pub const BORDER: Color32 = Color32::from_rgb(157, 157, 157);
pub const TEXT: Color32 = Color32::from_rgb(39, 39, 39);
pub const MUTED: Color32 = Color32::from_rgb(102, 102, 102);
pub const SELECT: Color32 = Color32::from_rgb(237, 157, 30);
pub const ACCENT: Color32 = Color32::from_rgb(127, 37, 41);
pub const DOT_BAND: Color32 = Color32::from_rgb(31, 52, 64);
pub const DOT_GUARD: Color32 = Color32::from_rgb(171, 51, 67);
pub const DOT_PERCUSSION: Color32 = Color32::from_rgb(49, 108, 131);
pub const DOT_OTHER: Color32 = Color32::from_rgb(91, 89, 111);
pub const BOX_SELECT_FILL: Color32 = Color32::from_rgba_premultiplied(237, 157, 30, 24);
pub const PREVIEW_SURROUND: Color32 = Color32::from_rgb(43, 128, 51);
pub const PREVIEW_FIELD: Color32 = Color32::from_rgb(38, 117, 45);
pub const PREVIEW_LINE: Color32 = Color32::from_gray(220);
pub const PREVIEW_BORDER: Color32 = Color32::WHITE;
pub const GUIDE: Color32 = Color32::from_rgb(64, 113, 126);
pub const COMPASS_PREVIEW: Color32 = Color32::from_rgb(127, 37, 41);

pub const FONT_CHROME: f32 = 11.0;
pub const FONT_SMALL: f32 = 10.0;
pub const FONT_FIELD: f32 = 18.0;
pub const FONT_DOT: f32 = 10.0;
pub const MENU_HEIGHT: f32 = 21.0;
pub const TOOLBAR_HEIGHT: f32 = 27.0;
pub const OPTIONS_HEIGHT: f32 = 25.0;
pub const COMMAND_PANEL_WIDTH: f32 = 350.0;
pub const ICON_INSET: f32 = 4.0;
pub const TOOL_HOVER: Color32 = Color32::from_gray(201);
pub const TOOL_ACTIVE: Color32 = Color32::from_gray(190);
pub const GROUP_GAP: f32 = 7.0;
pub const TIMELINE_HEIGHT: f32 = 68.0;
pub const STATUS_HEIGHT: f32 = 18.0;
pub const TOOL_SIZE: f32 = 20.0;
pub const CHROME_PAD: f32 = 3.0;
pub const INTERACT_WIDTH: f32 = 18.0;
pub const INTERACT_HEIGHT: f32 = 17.0;
pub const MENU_MARGIN: i8 = 2;
pub const SLIDER_WIDTH: f32 = 90.0;
pub const PANEL_WIDTH: f32 = 270.0;
pub const PANEL_HEIGHT: f32 = 190.0;
pub const PREVIEW_WIDTH: f32 = 252.0;
pub const PREVIEW_HEIGHT: f32 = 150.0;
pub const PALETTE_TITLE_HEIGHT: f32 = 16.0;
pub const PALETTE_CLOSE_SIZE: f32 = 14.0;
pub const CORNER_RADIUS: u8 = 0;
pub const BORDER_WIDTH: f32 = 1.0;
pub const GRID_MINOR_WIDTH: f32 = 0.5;
pub const GRID_MAJOR_WIDTH: f32 = 0.9;
pub const DOT_RADIUS: f32 = 3.5;
pub const SELECT_RADIUS: f32 = 5.0;
pub const SELECT_OUTER_RADIUS_OFFSET: f32 = 2.0;
pub const LABEL_ZOOM_THRESHOLD: f32 = 3.4;
pub const HIT_RADIUS: f32 = 8.0;
pub const CANVAS_INSET: f32 = 18.0;
pub const PALETTE_TOP_GAP: f32 = 6.0;
pub const PALETTE_RIGHT_GAP: f32 = 10.0;
pub const PREVIEW_START_X: f32 = 420.0;
pub const PREVIEW_TOP_INSET: f32 = 22.0;
pub const PREVIEW_BOTTOM_INSET: f32 = 12.0;
pub const PREVIEW_TOP_SIDE_INSET: f32 = 58.0;
pub const PREVIEW_BOTTOM_SIDE_INSET: f32 = 14.0;
pub const PREVIEW_DOT_RADIUS: f32 = 1.5;
pub const TIMELINE_LEFT_INSET: f32 = 28.0;
pub const TIMELINE_RIGHT_INSET: f32 = 8.0;
pub const TIMELINE_MINOR_TICK: f32 = 4.0;
pub const TIMELINE_MAJOR_TICK: f32 = 8.0;
pub const TIMELINE_LABEL_GAP: f32 = 11.0;
pub const TIMELINE_EDGE_GAP: f32 = 2.0;
pub const TIMELINE_PLAYHEAD_HALF: f32 = 4.0;
pub const TIMELINE_PLAYHEAD_POINT: f32 = 6.0;
pub const GRID_FORM_X_SPACING: f32 = 5.0;
pub const GRID_FORM_Y_SPACING: f32 = 2.0;
pub const WINDOW_WIDTH: f32 = 1280.0;
pub const WINDOW_HEIGHT: f32 = 800.0;
pub const SHORTCUT_WINDOW_WIDTH: f32 = 500.0;
pub const SHORTCUT_WINDOW_HEIGHT: f32 = 520.0;
pub const SHORTCUT_BINDING_WIDTH: f32 = 120.0;
pub const SHORTCUT_CLEAR_WIDTH: f32 = 42.0;
pub const SHAPE_BUTTON_WIDTH: f32 = 78.0;
pub const HUB_SIDEBAR_WIDTH: f32 = 190.0;
pub const HUB_HEADER_HEIGHT: f32 = 32.0;
pub const HUB_COMMAND_HEIGHT: f32 = 25.0;
pub const HUB_RECENT_ROW_HEIGHT: f32 = 34.0;
pub const HUB_MARK_SIZE: f32 = 42.0;
pub const HUB_CONTENT_WIDTH: f32 = 760.0;
pub const COMPASS_WINDOW_WIDTH: f32 = 360.0;
pub const COMPASS_WINDOW_HEIGHT: f32 = 360.0;
pub const ZERO_RADIUS: f32 = 0.0;
pub const HAIRLINE_WIDTH: f32 = 0.5;
pub const NORMAL_LINE_WIDTH: f32 = 1.0;
pub const EMPHASIS_LINE_WIDTH: f32 = 1.5;
pub const SET_MARKER_WIDTH: f32 = 2.0;
pub const LABEL_OFFSET: f32 = 5.0;
pub const HASH_HALF_LENGTH: f32 = 4.0;
pub const FIELD_NUMBER_VERTICAL_OFFSET: f32 = 0.20;
pub const YARD_MAJOR_WIDTH: f32 = 1.1;
pub const YARD_MINOR_WIDTH: f32 = 0.7;
pub const HASH_LINE_WIDTH: f32 = 0.8;
pub const FIELD_BORDER_WIDTH: f32 = 1.2;
pub const SELECT_STROKE_WIDTH: f32 = 1.2;
pub const GUIDE_WIDTH: f32 = 1.0;
pub const GUIDE_SELECTED_WIDTH: f32 = 1.8;
pub const GUIDE_HANDLE_RADIUS: f32 = 3.0;
pub const GUIDE_SPOT_RADIUS: f32 = 1.8;
pub const GUIDE_HIT_PIXELS: f32 = 7.0;
pub const CONSTRUCTION_DASH: f32 = 6.0;
pub const CONSTRUCTION_GAP: f32 = 4.0;
pub const COMPASS_TARGET_RADIUS: f32 = 4.5;
pub const PICKER_OFFSET: f32 = 8.0;
pub const PANEL_SHADOW: egui::Shadow = egui::Shadow {
    offset: [1, 1],
    blur: 2,
    spread: 0,
    color: Color32::from_black_alpha(56),
};

pub fn palette_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(CHROME_LIGHT)
        .stroke(Stroke::new(BORDER_WIDTH, BORDER))
        .inner_margin(egui::Margin::same(CHROME_PAD as i8))
        .shadow(PANEL_SHADOW)
}

pub fn chrome_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(CHROME)
        .stroke(Stroke::new(BORDER_WIDTH, BORDER))
        .inner_margin(egui::Margin::symmetric(CHROME_PAD as i8, 0))
}

pub fn apply(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::light();
    visuals.override_text_color = Some(TEXT);
    visuals.panel_fill = CHROME;
    visuals.window_fill = CHROME_LIGHT;
    visuals.window_stroke = Stroke::new(BORDER_WIDTH, BORDER);
    visuals.window_corner_radius = egui::CornerRadius::same(CORNER_RADIUS);
    visuals.menu_corner_radius = egui::CornerRadius::same(CORNER_RADIUS);
    visuals.window_shadow = PANEL_SHADOW;
    visuals.popup_shadow = egui::Shadow::NONE;
    visuals.extreme_bg_color = CANVAS;
    visuals.text_edit_bg_color = Some(CANVAS);
    visuals.selection.bg_fill = SELECT;
    for widget in [
        &mut visuals.widgets.noninteractive,
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        widget.corner_radius = egui::CornerRadius::same(CORNER_RADIUS);
        widget.bg_fill = CHROME_LIGHT;
        widget.weak_bg_fill = CHROME_LIGHT;
        widget.bg_stroke = Stroke::new(BORDER_WIDTH, BORDER);
        widget.fg_stroke = Stroke::new(BORDER_WIDTH, TEXT);
        widget.expansion = 0.0;
    }
    visuals.widgets.hovered.bg_fill = CHROME_DARK;
    visuals.widgets.active.bg_fill = CHROME_DARK;
    visuals.widgets.inactive.weak_bg_fill = CHROME;
    visuals.widgets.inactive.bg_stroke = Stroke::new(HAIRLINE_WIDTH, BORDER);
    visuals.widgets.open.bg_fill = TOOL_ACTIVE;
    visuals.selection.bg_fill = TOOL_ACTIVE;
    ctx.set_visuals(visuals);
    ctx.all_styles_mut(|style| {
        style.spacing.item_spacing = Vec2::splat(CHROME_PAD);
        style.spacing.button_padding = Vec2::new(CHROME_PAD, NORMAL_LINE_WIDTH);
        style.spacing.interact_size = Vec2::new(INTERACT_WIDTH, INTERACT_HEIGHT);
        style.spacing.window_margin = egui::Margin::same(CHROME_PAD as i8);
        style.spacing.menu_margin = egui::Margin::same(MENU_MARGIN);
        style.spacing.slider_width = SLIDER_WIDTH;
        style
            .text_styles
            .insert(TextStyle::Body, FontId::proportional(FONT_CHROME));
        style
            .text_styles
            .insert(TextStyle::Button, FontId::proportional(FONT_CHROME));
        style
            .text_styles
            .insert(TextStyle::Heading, FontId::proportional(FONT_CHROME));
        style
            .text_styles
            .insert(TextStyle::Small, FontId::proportional(FONT_SMALL));
    });
}
