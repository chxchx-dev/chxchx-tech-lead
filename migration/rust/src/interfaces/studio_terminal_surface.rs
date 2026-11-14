//! The widget that makes a terminal a terminal: it takes the focus on click, receives the keys
//! while it has it (so Studio's own shortcuts do not also fire), scrolls the history with the wheel
//! and reports its size so the program in it can be told how many rows and columns it has.
use super::design;
use super::terminal_keys;
use iced::advanced::widget::operation::Focusable;
use iced::advanced::widget::{Id, Operation, Tree, tree};
use iced::advanced::{Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer};
use iced::{Border, Element, Event, Length, Rectangle, Size, Vector, keyboard, window};
use std::sync::atomic::{AtomicBool, Ordering};

/// Read by the global shortcuts: while the terminal has the focus they must not also fire.
static CAPTURING: AtomicBool = AtomicBool::new(false);

pub(super) fn capturing() -> bool {
    CAPTURING.load(Ordering::Relaxed)
}

type Callback<'a, Message, Argument> = Box<dyn Fn(Argument) -> Message + 'a>;

pub(super) struct Surface<'a, Message> {
    id: Id,
    content: Element<'a, Message>,
    focused: bool,
    on_focus: Callback<'a, Message, bool>,
    on_key: Callback<'a, Message, keyboard::Event>,
    on_scroll: Callback<'a, Message, i32>,
    on_size: Callback<'a, Message, Size>,
}

impl<'a, Message> Surface<'a, Message> {
    pub fn new(
        id: Id,
        content: impl Into<Element<'a, Message>>,
        focused: bool,
        on_focus: impl Fn(bool) -> Message + 'a,
        on_key: impl Fn(keyboard::Event) -> Message + 'a,
        on_scroll: impl Fn(i32) -> Message + 'a,
        on_size: impl Fn(Size) -> Message + 'a,
    ) -> Self {
        Self {
            id,
            content: content.into(),
            focused,
            on_focus: Box::new(on_focus),
            on_key: Box::new(on_key),
            on_scroll: Box::new(on_scroll),
            on_size: Box::new(on_size),
        }
    }
}

#[derive(Default)]
struct State {
    focused: bool,
    size: Size,
}

impl Drop for State {
    /// The surface left the screen (the dock was hidden) while focused: Studio's shortcuts must come back.
    fn drop(&mut self) {
        if self.focused {
            CAPTURING.store(false, Ordering::Relaxed);
        }
    }
}

impl Focusable for State {
    fn is_focused(&self) -> bool {
        self.focused
    }

    fn focus(&mut self) {
        self.focused = true;
        CAPTURING.store(true, Ordering::Relaxed);
    }

    fn unfocus(&mut self) {
        self.focused = false;
        CAPTURING.store(false, Ordering::Relaxed);
    }
}

/// Lines of history a wheel notch moves.
const WHEEL_LINES: f32 = 3.0;

impl<Message: Clone> Widget<Message, iced::Theme, iced::Renderer> for Surface<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        let state = tree.state.downcast_mut::<State>();
        operation.focusable(Some(&self.id), layout.bounds(), state);
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
        let state = tree.state.downcast_mut::<State>();
        let bounds = layout.bounds();
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let inside = cursor.is_over(bounds);
                if inside != state.focused {
                    state.focused = inside;
                    CAPTURING.store(inside, Ordering::Relaxed);
                    shell.publish((self.on_focus)(inside));
                }
                if inside {
                    shell.capture_event();
                }
            }
            Event::Mouse(mouse::Event::WheelScrolled { delta }) if cursor.is_over(bounds) => {
                let lines = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => y * WHEEL_LINES,
                    mouse::ScrollDelta::Pixels { y, .. } => y / 16.0,
                };
                if lines.abs() >= 1.0 {
                    shell.publish((self.on_scroll)(lines as i32));
                }
                shell.capture_event();
            }
            Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. })
                if state.focused && !terminal_keys::reserved(key, *modifiers) =>
            {
                shell.publish((self.on_key)(event_key(event)));
                shell.capture_event();
            }
            Event::Window(window::Event::RedrawRequested(_)) => {
                if state.focused != self.focused {
                    // The focus changed without a click (a session was just opened): tell the app.
                    shell.publish((self.on_focus)(state.focused));
                }
                let size = bounds.size();
                if size != state.size {
                    state.size = size;
                    shell.publish((self.on_size)(size));
                }
            }
            _ => {}
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        let inner = self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        );
        if inner == mouse::Interaction::None && cursor.is_over(layout.bounds()) {
            mouse::Interaction::Text
        } else {
            inner
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
        if tree.state.downcast_ref::<State>().focused {
            iced::advanced::Renderer::fill_quad(
                renderer,
                renderer::Quad {
                    bounds: layout.bounds(),
                    border: Border {
                        color: design::alpha(design::ACCENT, 0.55),
                        width: 1.0,
                        radius: 8.0.into(),
                    },
                    ..Default::default()
                },
                iced::Color::TRANSPARENT,
            );
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, iced::Theme, iced::Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}

fn event_key(event: &Event) -> keyboard::Event {
    match event {
        Event::Keyboard(inner) => inner.clone(),
        _ => unreachable!("only keyboard events are forwarded"),
    }
}

impl<'a, Message: Clone + 'a> From<Surface<'a, Message>> for Element<'a, Message> {
    fn from(surface: Surface<'a, Message>) -> Self {
        Element::new(surface)
    }
}
