//! Windows: notices the start of a drag (left button pressed on the content
//! and the mouse moved a few pixels) and reports it once, for 2fip to run
//! the drag itself. libcosmic's own drag source can't be used there: it has
//! no drag and drop on Windows, never hears that the drag ended, and keeps
//! asking for the "grabbing" cursor (four arrows) over the whole window.
#![cfg_attr(not(windows), allow(dead_code))]

use cosmic::iced::advanced::layout::{self, Layout};
use cosmic::iced::advanced::renderer;
use cosmic::iced::advanced::widget::{tree, Operation, Tree};
use cosmic::iced::advanced::{overlay, Clipboard, Shell, Widget};
use cosmic::iced::{mouse, Event, Length, Point, Rectangle, Size, Vector};
use cosmic::{Element, Renderer, Theme};

/// How far the mouse must move before a press becomes a drag, in pixels
/// (as libcosmic's drag source).
const DRAG_THRESHOLD: f32 = 8.0;

pub struct DragStart<'a, Message> {
    content: Element<'a, Message>,
    on_drag: Message,
}

/// Wraps `content`: `on_drag` is published when a drag starts on it.
pub fn drag_start<'a, Message>(
    content: impl Into<Element<'a, Message>>,
    on_drag: Message,
) -> DragStart<'a, Message> {
    DragStart {
        content: content.into(),
        on_drag,
    }
}

#[derive(Default)]
struct State {
    /// Where the left button went down on the content, until it's released
    /// or the drag starts.
    pressed_at: Option<Point>,
}

impl<Message: Clone> Widget<Message, Theme, Renderer> for DragStart<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&mut self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_mut(&mut self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
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
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
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
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        // Watched before the content sees the event, and never captured:
        // clicks still select, open and so on as usual.
        let state = tree.state.downcast_mut::<State>();
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                state.pressed_at = cursor.position_over(layout.bounds());
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                state.pressed_at = None;
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                if let Some(start) = state.pressed_at {
                    if position.distance(start) > DRAG_THRESHOLD {
                        // Once per press: a Windows drag swallows the
                        // release, so don't wait for it.
                        state.pressed_at = None;
                        shell.publish(self.on_drag.clone());
                    }
                }
            }
            _ => {}
        }
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
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
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
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }

    fn drag_destinations(
        &self,
        state: &Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        dnd_rectangles: &mut cosmic::iced::advanced::clipboard::DndDestinationRectangles,
    ) {
        self.content.as_widget().drag_destinations(
            &state.children[0],
            layout,
            renderer,
            dnd_rectangles,
        );
    }
}

impl<'a, Message: Clone + 'a> From<DragStart<'a, Message>> for Element<'a, Message> {
    fn from(drag: DragStart<'a, Message>) -> Self {
        Element::new(drag)
    }
}
