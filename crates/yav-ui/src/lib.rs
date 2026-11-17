//! # YAV UI
//!
//! Official UI component library for the Yav web framework.
//!
//! Provides accessible, customizable, server-rendered components styled with
//! Tailwind CSS utility classes and design tokens.
//!
//! ## Components
//! - **Button**: [`struct@Button`]
//! - **Card**: [`struct@Card`], [`struct@CardHeader`], [`struct@CardTitle`], [`struct@CardDescription`], [`struct@CardContent`], [`struct@CardFooter`]
//! - **Input & Form Controls**: [`struct@Input`], [`struct@Textarea`], [`struct@Select`], [`struct@Checkbox`], [`struct@Radio`], [`struct@Slider`], [`struct@RangeSlider`], [`struct@Switch`]
//! - **Layout**: [`struct@Container`], [`struct@Stack`], [`struct@Grid`], [`struct@Separator`]
//! - **Navigation**: [`struct@Link`], [`struct@Navbar`], [`struct@Sidebar`], [`struct@Breadcrumbs`], [`struct@Tabs`]
//! - **Feedback**: [`struct@Alert`], [`struct@Badge`], [`struct@Loading`], [`struct@Skeleton`], [`struct@ProgressBar`], [`struct@EmptyState`]
//! - **Overlay**: [`struct@Dialog`], [`struct@Drawer`], [`struct@Dropdown`], [`struct@Popover`], [`struct@Tooltip`]
//! - **Data Display**: [`struct@Table`], [`struct@Pagination`], [`struct@Avatar`]
//! - **Forms**: [`struct@Form`], [`struct@Label`], [`struct@FormGroup`]
//! - **Syntax Highlighting**: [`struct@CodeBlock`]
//! - **Cursor & 3D Interactive**: [`struct@CursorSpotlight`], [`struct@TiltPlate`], [`struct@Magnetic`], [`struct@CursorFollower`]
//! - **Motion & Spring Physics**: [`struct@Motion`], [`struct@CountUp`], [`struct@AnimatedBar`], [`struct@TypewriterCode`], [`struct@ScrollProgressBar`]
//! - **Multi-Theme System**: [`Theme`], [`ThemePreset`], [`struct@ThemeSelector`], [`struct@ThemeToggle`]

pub mod ai_filter_bar;
pub mod audio_waveform;
pub mod button;
pub mod calendar;
pub mod card;
pub mod carousel;
pub mod charts;
pub mod charts_advanced;
pub mod code_block;
pub mod code_playground;
pub mod command;
pub mod cookie_consent;
pub mod cursor;
pub mod data;
pub mod datagrid;
pub mod devtools;
pub mod feedback;
pub mod floating;
pub mod flow_canvas;
pub mod font;
pub mod form;
pub mod form_builder;
pub mod gantt_chart;
pub mod hex_grid;
pub mod image;
pub mod image_compare;
pub mod input;
pub mod islands;
pub mod json_tree;
pub mod kanban;
pub mod layout;
pub mod log_viewer;
pub mod markdown_editor;
pub mod media_diff;
pub mod media_recorder;
pub mod meta;
pub mod model_viewer_3d;
pub mod motion;
pub mod navigation;
pub mod org_chart;
pub mod overlay;
pub mod pdf_viewer;
pub mod rich_editor;
pub mod signature_pad;
pub mod spotlight_tour;
pub mod spreadsheet;
pub mod stepper;
pub mod terminal;
pub mod theme;
pub mod timeline;
pub mod toast;
pub mod utils;
pub mod vector_map;
pub mod virtual_list;
pub mod whiteboard;
pub mod widgets;

// Re-exports
pub use ai_filter_bar::{AiFilterBar, AiFilterToken};
pub use audio_waveform::AudioWaveform;
pub use button::{button, Button, ButtonProps};
pub use calendar::{Calendar, CalendarEvent, CalendarProps, Scheduler, TimeSlot};
pub use card::{
    card, card_content, card_description, card_footer, card_header, card_title, stat_card, Card,
    CardContent, CardContentProps, CardDescription, CardDescriptionProps, CardFooter,
    CardFooterProps, CardHeader, CardHeaderProps, CardProps, CardTitle, CardTitleProps, StatCard,
    StatCardProps, StatTrend, StatTrendDirection,
};
pub use carousel::{carousel, carousel_script, Carousel, CarouselProps, CarouselSlide};
pub use charts::{
    bar_chart, donut_chart, line_chart, sparkline, BarChart, BarChartProps, ChartPoint, DonutChart,
    DonutChartProps, LineChart, LineChartProps, Sparkline, SparklineProps,
};
pub use charts_advanced::{Heatmap, HeatmapCell, RadarChart, SankeyChart, SankeyLink, SankeyNode};
pub use code_block::{code_block, CodeBlock, CodeBlockProps, CodeLanguage};
pub use code_playground::{code_playground, CodePlayground, PlaygroundTheme};
pub use command::{
    command_menu, command_palette_script, CommandGroup, CommandGroupProps, CommandItem,
    CommandItemProps, CommandMenu, CommandMenuProps,
};
pub use cookie_consent::{cookie_consent, CookieConsent};
pub use cursor::{
    cursor_follower, cursor_spotlight, magnetic, tilt_plate, CursorFollower, CursorFollowerProps,
    CursorSpotlight, CursorSpotlightProps, Magnetic, MagneticProps, TiltPlate, TiltPlateProps,
};
pub use data::{
    avatar, pagination, table, table_cell, table_header_cell, table_row, Avatar, AvatarProps,
    AvatarStatus, CellAlign, Pagination, PaginationProps, Table, TableCell, TableProps, TableRow,
};
pub use datagrid::{data_grid, DataGrid, DataGridColumn, DataGridProps};
pub use devtools::{DevTools, DevToolsProps};
pub use feedback::{
    alert, badge, empty_state, loading, progress_bar, skeleton, Alert, AlertProps, AlertVariant,
    Badge, BadgeProps, BadgeVariant, EmptyState, EmptyStateProps, Loading, LoadingProps,
    ProgressBar, ProgressBarProps, Skeleton, SkeletonProps,
};
pub use floating::{
    floating_script, popover, tooltip, Placement, Popover, PopoverProps, Tooltip, TooltipProps,
};
pub use flow_canvas::{FlowCanvas, FlowCanvasProps, FlowEdge, FlowNode};
pub use font::{FallbackMetrics, FontDisplay, GoogleFont, LocalFont};
pub use form::{
    form, form_group, label, CsrfToken, Form, FormField, FormFieldProps, FormGroup, FormGroupProps,
    FormMethod, FormProps, Label, LabelProps,
};
pub use form_builder::DynamicForm;
pub use gantt_chart::{gantt_chart, GanttChart, GanttTask, GanttViewMode};
pub use hex_grid::{hex_grid, HexCoord, HexGrid, HexTile};
pub use image::{image, Image, ImageProps};
pub use image_compare::{image_compare_slider, ImageCompareSlider};
pub use input::{
    checkbox, input, radio, range_slider, select, slider, switch, textarea, Checkbox,
    CheckboxProps, Input, InputProps, Radio, RadioProps, RangeSlider, RangeSliderProps, Select,
    SelectOption, SelectProps, Slider, SliderProps, Switch, SwitchProps, Textarea, TextareaProps,
};
pub use islands::{
    web_components_script, Accordion, AccordionItem, AccordionItemProps, AccordionProps,
    ClientDirective, Counter, CounterProps, Island, IslandProps,
};
pub use json_tree::{json_tree_viewer, JsonTreeViewer};
pub use kanban::{KanbanBoard, KanbanBoardProps, KanbanCard, KanbanColumn};
pub use layout::{
    container, grid, hstack, separator, stack, vstack, Container, ContainerProps, ContainerSize,
    Grid, GridProps, Orientation, Separator, SeparatorProps, Stack, StackAlign, StackDirection,
    StackJustify, StackProps,
};
pub use log_viewer::{LogEntry, LogLevel, LogViewer};
pub use markdown_editor::{sanitize_preview_html, MarkdownEditor};
pub use media_diff::{
    compute_line_diff, DiffLine, DiffViewer, MediaChapter, MediaPlayer, SubtitleTrack,
};
pub use media_recorder::{media_recorder, AudioFormat, MediaRecorder};
pub use meta::{MetaTags, MetaTagsProps, Metadata, TwitterCard};
pub use model_viewer_3d::{model_viewer_3d, ModelMesh3D, ModelViewer3D, Point3D};
pub use motion::{
    animated_bar, count_up, motion, motion_group, motion_script, scroll_progress_bar,
    typewriter_code, AnimatedBar, AnimatedBarProps, CountUp, CountUpProps, Motion, MotionGroup,
    MotionGroupProps, MotionProps, MotionState, ScrollProgressBar, ScrollProgressBarProps,
    Transition, TypewriterCode, TypewriterCodeProps,
};
pub use navigation::{
    breadcrumbs, client_router_script, link, nav_item, navbar, navbar_brand, sidebar, sidebar_item,
    tabs, BreadcrumbItem, Breadcrumbs, BreadcrumbsProps, Link, LinkProps, NavItem, Navbar,
    NavbarBrand, NavbarProps, Sidebar, SidebarItem, SidebarItemProps, SidebarProps, TabItem, Tabs,
    TabsProps,
};
pub use org_chart::{OrgChart, OrgChartProps, OrgNode, OrgOrientation};
pub use overlay::{
    dialog, drawer, dropdown, dropdown_item, Dialog, DialogProps, Drawer, DrawerPlacement,
    DrawerProps, Dropdown, DropdownItem, DropdownProps,
};
pub use pdf_viewer::{pdf_viewer, PdfAnnotation, PdfViewer};
pub use rich_editor::{AiChat, AiChatBubble, AiChatProps, RichTextEditor, RichTextEditorProps};
pub use signature_pad::{signature_pad, SignaturePad};
pub use spotlight_tour::{spotlight_tour, SpotlightTour, TourPlacement, TourStep};
pub use spreadsheet::Spreadsheet;
pub use stepper::{stepper, StepItem, StepStatus, Stepper, StepperProps};
pub use terminal::{parse_ansi, AnsiSpan, Terminal, TerminalProps};
pub use theme::{
    anti_fouc_script, reduced_motion_style, theme_selector, theme_toggle, BorderRadii,
    ColorPalette, SpacingScale, Theme, ThemePreset, ThemeSelector, ThemeSelectorProps, ThemeToggle,
    ThemeToggleProps, Typography,
};
pub use timeline::{timeline, Timeline, TimelineItem, TimelineStatus};
pub use toast::{toast_script, Toast, ToastContainer, ToastContainerProps, ToastLevel, ToastProps};
pub use vector_map::{vector_map, MapPin, MapRegion, VectorMap};
pub use virtual_list::{VirtualList, VirtualListProps};
pub use whiteboard::{Whiteboard, WhiteboardElement, WhiteboardTool};
pub use widgets::{
    Combobox, ComboboxProps, DatePicker, DatePickerProps, FileDropzone, FileDropzoneProps,
    InputOtp, InputOtpProps, Kbd, KbdProps, ResizablePanels, ResizablePanelsProps, Sheet,
    SheetProps,
};

/// Visual variant styling for buttons, badges, and alerts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Variant {
    #[default]
    Primary,
    Secondary,
    Outline,
    Ghost,
    Danger,
    Success,
    Warning,
}

/// Type alias for [`Variant`] when used in button contexts.
pub type ButtonVariant = Variant;

/// Sizing scale for buttons, inputs, badges, and avatars.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Size {
    Sm,
    #[default]
    Md,
    Lg,
    Xl,
}

/// Convenience prelude module bringing common UI components and types into scope.
pub mod prelude {
    pub use crate::ai_filter_bar::{AiFilterBar, AiFilterToken};
    pub use crate::audio_waveform::AudioWaveform;
    pub use crate::button::{button, Button, ButtonProps};
    pub use crate::calendar::{Calendar, CalendarEvent, CalendarProps, Scheduler, TimeSlot};
    pub use crate::card::{
        card, stat_card, Card, CardContent, CardContentProps, CardDescription,
        CardDescriptionProps, CardFooter, CardFooterProps, CardHeader, CardHeaderProps, CardProps,
        CardTitle, CardTitleProps, StatCard, StatCardProps, StatTrend, StatTrendDirection,
    };
    pub use crate::carousel::{carousel, carousel_script, Carousel, CarouselProps, CarouselSlide};
    pub use crate::charts::{
        bar_chart, donut_chart, line_chart, sparkline, BarChart, BarChartProps, ChartPoint,
        DonutChart, DonutChartProps, LineChart, LineChartProps, Sparkline, SparklineProps,
    };
    pub use crate::charts_advanced::{
        Heatmap, HeatmapCell, RadarChart, SankeyChart, SankeyLink, SankeyNode,
    };
    pub use crate::code_block::{code_block, CodeBlock, CodeBlockProps, CodeLanguage};
    pub use crate::code_playground::{code_playground, CodePlayground, PlaygroundTheme};
    pub use crate::command::{
        command_menu, command_palette_script, CommandGroup, CommandGroupProps, CommandItem,
        CommandItemProps, CommandMenu, CommandMenuProps,
    };
    pub use crate::cookie_consent::{cookie_consent, CookieConsent};
    pub use crate::cursor::{
        cursor_follower, cursor_spotlight, magnetic, tilt_plate, CursorFollower,
        CursorFollowerProps, CursorSpotlight, CursorSpotlightProps, Magnetic, MagneticProps,
        TiltPlate, TiltPlateProps,
    };
    pub use crate::data::{
        avatar, pagination, table, Avatar, AvatarProps, Pagination, PaginationProps, Table,
        TableCell, TableProps, TableRow,
    };
    pub use crate::datagrid::{data_grid, DataGrid, DataGridColumn, DataGridProps};
    pub use crate::devtools::{DevTools, DevToolsProps};
    pub use crate::feedback::{
        alert, badge, empty_state, loading, progress_bar, skeleton, Alert, AlertProps, Badge,
        BadgeProps, EmptyState, EmptyStateProps, Loading, LoadingProps, ProgressBar,
        ProgressBarProps, Skeleton, SkeletonProps,
    };
    pub use crate::floating::{
        floating_script, popover, tooltip, Placement, Popover, PopoverProps, Tooltip, TooltipProps,
    };
    pub use crate::flow_canvas::{FlowCanvas, FlowCanvasProps, FlowEdge, FlowNode};
    pub use crate::font::{FallbackMetrics, FontDisplay, GoogleFont, LocalFont};
    pub use crate::form::{
        form, form_group, label, CsrfToken, Form, FormField, FormFieldProps, FormGroup,
        FormGroupProps, FormMethod, FormProps, Label, LabelProps,
    };
    pub use crate::form_builder::DynamicForm;
    pub use crate::gantt_chart::{gantt_chart, GanttChart, GanttTask, GanttViewMode};
    pub use crate::hex_grid::{hex_grid, HexCoord, HexGrid, HexTile};
    pub use crate::image::{image, Image, ImageProps};
    pub use crate::image_compare::{image_compare_slider, ImageCompareSlider};
    pub use crate::input::{
        checkbox, input, radio, range_slider, select, slider, switch, textarea, Checkbox,
        CheckboxProps, Input, InputProps, Radio, RadioProps, RangeSlider, RangeSliderProps, Select,
        SelectOption, SelectProps, Slider, SliderProps, Switch, SwitchProps, Textarea,
        TextareaProps,
    };
    pub use crate::islands::{
        web_components_script, Accordion, AccordionItem, AccordionItemProps, AccordionProps,
        ClientDirective, Counter, CounterProps, Island, IslandProps,
    };
    pub use crate::json_tree::{json_tree_viewer, JsonTreeViewer};
    pub use crate::kanban::{KanbanBoard, KanbanBoardProps, KanbanCard, KanbanColumn};
    pub use crate::layout::{
        container, grid, hstack, separator, stack, vstack, Container, ContainerProps,
        ContainerSize, Grid, GridProps, Orientation, Separator, SeparatorProps, Stack,
        StackDirection, StackProps,
    };
    pub use crate::log_viewer::{LogEntry, LogLevel, LogViewer};
    pub use crate::markdown_editor::MarkdownEditor;
    pub use crate::media_diff::{
        compute_line_diff, DiffLine, DiffViewer, MediaChapter, MediaPlayer, SubtitleTrack,
    };
    pub use crate::media_recorder::{media_recorder, AudioFormat, MediaRecorder};
    pub use crate::meta::{MetaTags, MetaTagsProps, Metadata, TwitterCard};
    pub use crate::model_viewer_3d::{model_viewer_3d, ModelMesh3D, ModelViewer3D, Point3D};
    pub use crate::motion::{
        animated_bar, count_up, motion, motion_group, motion_script, scroll_progress_bar,
        typewriter_code, AnimatedBar, AnimatedBarProps, CountUp, CountUpProps, Motion, MotionGroup,
        MotionGroupProps, MotionProps, MotionState, ScrollProgressBar, ScrollProgressBarProps,
        Transition, TypewriterCode, TypewriterCodeProps,
    };
    pub use crate::navigation::{
        breadcrumbs, client_router_script, link, navbar, sidebar, tabs, Breadcrumbs,
        BreadcrumbsProps, Link, LinkProps, Navbar, NavbarProps, Sidebar, SidebarItem,
        SidebarItemProps, SidebarProps, Tabs, TabsProps,
    };
    pub use crate::org_chart::{OrgChart, OrgChartProps, OrgNode, OrgOrientation};
    pub use crate::overlay::{
        dialog, drawer, dropdown, Dialog, DialogProps, Drawer, DrawerPlacement, DrawerProps,
        Dropdown, DropdownItem, DropdownProps,
    };
    pub use crate::pdf_viewer::{pdf_viewer, PdfAnnotation, PdfViewer};
    pub use crate::rich_editor::{
        AiChat, AiChatBubble, AiChatProps, RichTextEditor, RichTextEditorProps,
    };
    pub use crate::signature_pad::{signature_pad, SignaturePad};
    pub use crate::spotlight_tour::{spotlight_tour, SpotlightTour, TourPlacement, TourStep};
    pub use crate::spreadsheet::Spreadsheet;
    pub use crate::stepper::{stepper, StepItem, StepStatus, Stepper, StepperProps};
    pub use crate::terminal::{parse_ansi, AnsiSpan, Terminal, TerminalProps};
    pub use crate::theme::{
        anti_fouc_script, reduced_motion_style, theme_selector, theme_toggle, Theme, ThemePreset,
        ThemeSelector, ThemeSelectorProps, ThemeToggle, ThemeToggleProps,
    };
    pub use crate::timeline::{timeline, Timeline, TimelineItem, TimelineStatus};
    pub use crate::toast::{
        toast_script, Toast, ToastContainer, ToastContainerProps, ToastLevel, ToastProps,
    };
    pub use crate::vector_map::{vector_map, MapPin, MapRegion, VectorMap};
    pub use crate::virtual_list::{VirtualList, VirtualListProps};
    pub use crate::whiteboard::{Whiteboard, WhiteboardElement, WhiteboardTool};
    pub use crate::widgets::{
        Combobox, ComboboxProps, DatePicker, DatePickerProps, FileDropzone, FileDropzoneProps,
        InputOtp, InputOtpProps, Kbd, KbdProps, ResizablePanels, ResizablePanelsProps, Sheet,
        SheetProps,
    };
    pub use crate::{ButtonVariant, Size, Variant};
    pub use yav_core::prelude::*;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_variant_defaults() {
        assert_eq!(Variant::default(), Variant::Primary);
    }

    #[test]
    fn test_size_defaults() {
        assert_eq!(Size::default(), Size::Md);
    }

    #[test]
    fn test_prelude_imports() {
        use crate::prelude::*;
        let btn = button("Hello").variant(Variant::Secondary).render();
        let html = render_to_string(&btn);
        assert!(html.contains("Hello"));
        assert!(html.contains("bg-gray-100"));
    }

    #[test]
    fn test_composite_page_rendering() {
        use crate::prelude::*;

        let page = container()
            .size(ContainerSize::Xl)
            .child(
                navbar()
                    .brand(NavbarBrand::new("/").label("Acme Inc"))
                    .item(NavItem::new("Home", "/").active(true))
                    .item(NavItem::new("Dashboard", "/dashboard")),
            )
            .child(
                stack()
                    .vertical()
                    .spacing(6)
                    .child(
                        alert()
                            .variant(Variant::Success)
                            .title("Welcome back!")
                            .message("Your account status is active."),
                    )
                    .child(
                        card()
                            .header(
                                CardHeader::new()
                                    .title(CardTitle::new("User Profile"))
                                    .description(CardDescription::new("Manage your details")),
                            )
                            .child(
                                CardContent::new().child(
                                    form()
                                        .action("/profile")
                                        .method(FormMethod::Post)
                                        .child(
                                            form_group(
                                                input()
                                                    .name("username")
                                                    .placeholder("Enter username"),
                                            )
                                            .label_text("Username")
                                            .help_text("Public display name"),
                                        )
                                        .child(
                                            button("Save Changes")
                                                .variant(Variant::Primary)
                                                .btn_type("submit"),
                                        ),
                                ),
                            ),
                    ),
            )
            .render();

        let html = render_to_string(&page);
        assert!(html.contains("Acme Inc"));
        assert!(html.contains("Welcome back!"));
        assert!(html.contains("User Profile"));
        assert!(html.contains("Public display name"));
        assert!(html.contains("Save Changes"));
    }
}
