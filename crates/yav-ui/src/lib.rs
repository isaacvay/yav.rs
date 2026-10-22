//! # YAV UI
//!
//! Official UI component library for the Yav web framework.
//!
//! Provides accessible, customizable, server-rendered components styled with
//! Tailwind CSS utility classes and design tokens.
//!
//! ## Components
//! - **Button**: [`Button`]
//! - **Card**: [`Card`], [`CardHeader`], [`CardTitle`], [`CardDescription`], [`CardContent`], [`CardFooter`]
//! - **Input & Form Controls**: [`Input`], [`Textarea`], [`Select`], [`Checkbox`], [`Radio`]
//! - **Layout**: [`Container`], [`Stack`], [`Grid`], [`Separator`]
//! - **Navigation**: [`Link`], [`Navbar`], [`Sidebar`], [`Breadcrumbs`], [`Tabs`]
//! - **Feedback**: [`Alert`], [`Badge`], [`Loading`], [`Skeleton`]
//! - **Overlay**: [`Dialog`], [`Dropdown`]
//! - **Data Display**: [`Table`], [`Pagination`], [`Avatar`]
//! - **Forms**: [`Form`], [`Label`], [`FormGroup`]
//! - **Theme**: [`Theme`], [`ColorPalette`], [`SpacingScale`], [`BorderRadii`], [`Typography`]

pub mod button;
pub mod card;
pub mod carousel;
pub mod command;
pub mod data;
pub mod feedback;
pub mod floating;
pub mod form;
pub mod image;
pub mod input;
pub mod islands;
pub mod layout;
pub mod meta;
pub mod motion;
pub mod navigation;
pub mod overlay;
pub mod stepper;
pub mod theme;
pub mod toast;
pub mod utils;

// Re-exports
pub use button::{button, Button, ButtonProps};
pub use card::{
    card, card_content, card_description, card_footer, card_header, card_title, stat_card, Card,
    CardContent, CardContentProps, CardDescription, CardDescriptionProps, CardFooter,
    CardFooterProps, CardHeader, CardHeaderProps, CardProps, CardTitle, CardTitleProps, StatCard,
    StatCardProps, StatTrend, StatTrendDirection,
};
pub use carousel::{carousel, carousel_script, Carousel, CarouselProps, CarouselSlide};
pub use command::{
    command_menu, command_palette_script, CommandGroup, CommandGroupProps, CommandItem,
    CommandItemProps, CommandMenu, CommandMenuProps,
};
pub use data::{
    avatar, pagination, table, table_cell, table_header_cell, table_row, Avatar, AvatarProps,
    AvatarStatus, CellAlign, Pagination, PaginationProps, Table, TableCell, TableProps, TableRow,
};
pub use feedback::{
    alert, badge, empty_state, loading, progress_bar, skeleton, Alert, AlertProps, AlertVariant,
    Badge, BadgeProps, BadgeVariant, EmptyState, EmptyStateProps, Loading, LoadingProps,
    ProgressBar, ProgressBarProps, Skeleton, SkeletonProps,
};
pub use floating::{
    floating_script, popover, tooltip, Placement, Popover, PopoverProps, Tooltip, TooltipProps,
};
pub use form::{
    form, form_group, label, CsrfToken, Form, FormField, FormFieldProps, FormGroup, FormGroupProps,
    FormMethod, FormProps, Label, LabelProps,
};
pub use image::{image, Image, ImageProps};
pub use input::{
    checkbox, input, radio, range_slider, select, slider, switch, textarea, Checkbox,
    CheckboxProps, Input, InputProps, Radio, RadioProps, RangeSlider, RangeSliderProps, Select,
    SelectOption, SelectProps, Slider, SliderProps, Switch, SwitchProps, Textarea, TextareaProps,
};
pub use islands::{
    web_components_script, Accordion, AccordionItem, AccordionItemProps, AccordionProps,
    ClientDirective, Counter, CounterProps, Island, IslandProps,
};
pub use layout::{
    container, grid, hstack, separator, stack, vstack, Container, ContainerProps, ContainerSize,
    Grid, GridProps, Orientation, Separator, SeparatorProps, Stack, StackAlign, StackDirection,
    StackJustify, StackProps,
};
pub use meta::{MetaTags, MetaTagsProps, Metadata, TwitterCard};
pub use motion::{
    motion, motion_group, motion_script, scroll_progress_bar, Motion, MotionGroup,
    MotionGroupProps, MotionProps, MotionState, ScrollProgressBar, ScrollProgressBarProps,
    Transition,
};
pub use navigation::{
    breadcrumbs, client_router_script, link, nav_item, navbar, navbar_brand, sidebar, sidebar_item,
    tabs, BreadcrumbItem, Breadcrumbs, BreadcrumbsProps, Link, LinkProps, NavItem, Navbar,
    NavbarBrand, NavbarProps, Sidebar, SidebarItem, SidebarItemProps, SidebarProps, TabItem, Tabs,
    TabsProps,
};
pub use overlay::{
    dialog, drawer, dropdown, dropdown_item, Dialog, DialogProps, Drawer, DrawerPlacement,
    DrawerProps, Dropdown, DropdownItem, DropdownProps,
};
pub use stepper::{stepper, StepItem, StepStatus, Stepper, StepperProps};
pub use theme::{
    anti_fouc_script, reduced_motion_style, theme_toggle, BorderRadii, ColorPalette, SpacingScale,
    Theme, ThemeToggle, ThemeToggleProps, Typography,
};
pub use toast::{toast_script, Toast, ToastContainer, ToastContainerProps, ToastLevel, ToastProps};

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
    pub use crate::button::{button, Button, ButtonProps};
    pub use crate::card::{
        card, stat_card, Card, CardContent, CardContentProps, CardDescription,
        CardDescriptionProps, CardFooter, CardFooterProps, CardHeader, CardHeaderProps, CardProps,
        CardTitle, CardTitleProps, StatCard, StatCardProps, StatTrend, StatTrendDirection,
    };
    pub use crate::carousel::{carousel, carousel_script, Carousel, CarouselProps, CarouselSlide};
    pub use crate::command::{
        command_menu, command_palette_script, CommandGroup, CommandGroupProps, CommandItem,
        CommandItemProps, CommandMenu, CommandMenuProps,
    };
    pub use crate::data::{
        avatar, pagination, table, Avatar, AvatarProps, Pagination, PaginationProps, Table,
        TableCell, TableProps, TableRow,
    };
    pub use crate::feedback::{
        alert, badge, empty_state, loading, progress_bar, skeleton, Alert, AlertProps, Badge,
        BadgeProps, EmptyState, EmptyStateProps, Loading, LoadingProps, ProgressBar,
        ProgressBarProps, Skeleton, SkeletonProps,
    };
    pub use crate::floating::{
        floating_script, popover, tooltip, Placement, Popover, PopoverProps, Tooltip, TooltipProps,
    };
    pub use crate::form::{
        form, form_group, label, CsrfToken, Form, FormField, FormFieldProps, FormGroup,
        FormGroupProps, FormMethod, FormProps, Label, LabelProps,
    };
    pub use crate::image::{image, Image, ImageProps};
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
    pub use crate::layout::{
        container, grid, hstack, separator, stack, vstack, Container, ContainerProps,
        ContainerSize, Grid, GridProps, Orientation, Separator, SeparatorProps, Stack,
        StackDirection, StackProps,
    };
    pub use crate::meta::{MetaTags, MetaTagsProps, Metadata, TwitterCard};
    pub use crate::motion::{
        motion, motion_group, motion_script, scroll_progress_bar, Motion, MotionGroup,
        MotionGroupProps, MotionProps, MotionState, ScrollProgressBar, ScrollProgressBarProps,
        Transition,
    };
    pub use crate::navigation::{
        breadcrumbs, client_router_script, link, navbar, sidebar, tabs, Breadcrumbs,
        BreadcrumbsProps, Link, LinkProps, Navbar, NavbarProps, Sidebar, SidebarItem,
        SidebarItemProps, SidebarProps, Tabs, TabsProps,
    };
    pub use crate::overlay::{
        dialog, drawer, dropdown, Dialog, DialogProps, Drawer, DrawerPlacement, DrawerProps,
        Dropdown, DropdownItem, DropdownProps,
    };
    pub use crate::stepper::{stepper, StepItem, StepStatus, Stepper, StepperProps};
    pub use crate::theme::{
        anti_fouc_script, reduced_motion_style, theme_toggle, Theme, ThemeToggle, ThemeToggleProps,
    };
    pub use crate::toast::{
        toast_script, Toast, ToastContainer, ToastContainerProps, ToastLevel, ToastProps,
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
