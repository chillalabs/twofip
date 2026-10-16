//! Rubber-band selection: press the left button on empty space in the file
//! list (not on a file) and drag; the files and folders the rectangle
//! touches get selected. With Ctrl held they are added to the selection.
//! A click on empty space, without dragging, clears the selection.
//!
//! The widget wraps the list *inside* its scroll area, so positions are in
//! list coordinates whatever the scroll position; it finds the entries from
//! the layout of the list it wraps.

use cosmic::iced::advanced::layout::{self, Layout};
use cosmic::iced::advanced::renderer;
use cosmic::iced::advanced::widget::{tree, Operation, Tree};
use cosmic::iced::advanced::{overlay, Clipboard, Shell, Widget};
use cosmic::iced::{keyboard, mouse, Border, Color, Event, Length, Point, Rectangle, Size, Vector};
use cosmic::{Element, Renderer, Theme};

/// What the rubber band reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    /// A drag (or a plain click) started on empty space; with `additive`
    /// (Ctrl held) the current selection is kept and added to.
    Started { additive: bool },
    /// The entries the rectangle touches now, by position in the list.
    Selected(Vec<usize>),
}

/// How far the mouse must move before a press becomes a drag, in pixels.
const DRAG_THRESHOLD: f32 = 4.0;

pub struct Marquee<'a, Message> {
    content: Element<'a, Message>,
    /// 1: the content's children are the entries (list view); 2: they are
    /// rows whose children are the entries (grid view).
    depth: usize,
    /// At least this tall, so the empty space below the last entry counts.
    min_height: f32,
    on_change: Box<dyn Fn(Change) -> Message + 'a>,
}

/// Wraps `content` (a list or grid of entries) with rubber-band selection.
pub fn marquee<'a, Message>(
    content: impl Into<Element<'a, Message>>,
    depth: usize,
    min_height: f32,
    on_change: impl Fn(Change) -> Message + 'a,
) -> Marquee<'a, Message> {
    Marquee {
        content: content.into(),
        depth,
        min_height,
        on_change: Box::new(on_change),
    }
}

#[derive(Default)]
struct State {
    /// Where the left button went down on empty space.
    start: Option<Point>,
    current: Point,
    dragging: bool,
    modifiers: keyboard::Modifiers,
    /// The entries reported last, so only changes are sent.
    last: Vec<usize>,
}

impl<Message> Marquee<'_, Message> {
    /// Positions of the entries whose layout touches `rect`.
    fn hits(&self, layout: Layout<'_>, rect: Rectangle) -> Vec<usize> {
        let Some(content) = layout.children().next() else {
            return Vec::new();
        };
        let entries: Box<dyn Iterator<Item = Layout<'_>>> = if self.depth >= 2 {
            Box::new(content.children().flat_map(|row| row.children()))
        } else {
            Box::new(content.children())
        };
        entries
            .enumerate()
            .filter(|(_, entry)| entry.bounds().intersects(&rect))
            .map(|(index, _)| index)
            .collect()
    }
}

fn rectangle(a: Point, b: Point) -> Rectangle {
    Rectangle {
        x: a.x.min(b.x),
        y: a.y.min(b.y),
        width: (a.x - b.x).abs(),
        height: (a.y - b.y).abs(),
    }
}

impl<Message> Widget<Message, Theme, Renderer> for Marquee<'_, Message> {
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
        Size::new(Length::Fill, Length::Shrink)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let content = self
            .content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits);
        let size = content.size();
        let width = limits.max().width.min(size.width.max(limits.min().width));
        let width = if width.is_finite() { width } else { size.width };
        layout::Node::with_children(
            Size::new(width.max(size.width), size.height.max(self.min_height)),
            vec![content],
        )
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn Operation,
    ) {
        if let Some(content) = layout.children().next() {
            self.content.as_widget_mut().operate(
                &mut tree.children[0],
                content,
                renderer,
                operation,
            );
        }
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
        if let Some(content) = layout.children().next() {
            self.content.as_widget_mut().update(
                &mut tree.children[0],
                event,
                content,
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            );
        }

        let bounds = layout.bounds();
        match event {
            Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) => {
                tree.state.downcast_mut::<State>().modifiers = *modifiers;
            }
            // A press a file didn't take: the start of a rubber band. Not
            // captured, so the panel still becomes the active one.
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if shell.is_event_captured() {
                    return;
                }
                if let Some(position) = cursor.position_over(bounds) {
                    let state = tree.state.downcast_mut::<State>();
                    state.start = Some(position);
                    state.current = position;
                    state.dragging = false;
                    state.last.clear();
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let hits = {
                    let state = tree.state.downcast_ref::<State>();
                    let (Some(start), Some(position)) = (state.start, cursor.position()) else {
                        return;
                    };
                    let position = Point::new(
                        position.x.clamp(bounds.x, bounds.x + bounds.width),
                        position.y.clamp(bounds.y, bounds.y + bounds.height),
                    );
                    if !state.dragging && start.distance(position) < DRAG_THRESHOLD {
                        return;
                    }
                    (start, position)
                };
                let (start, position) = hits;
                let selected = self.hits(layout, rectangle(start, position));
                let state = tree.state.downcast_mut::<State>();
                state.current = position;
                if !state.dragging {
                    state.dragging = true;
                    let additive = state.modifiers.control();
                    shell.publish((self.on_change)(Change::Started { additive }));
                }
                if selected != state.last {
                    state.last = selected.clone();
                    shell.publish((self.on_change)(Change::Selected(selected)));
                }
                shell.capture_event();
                shell.request_redraw();
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                let state = tree.state.downcast_mut::<State>();
                if state.start.take().is_none() {
                    return;
                }
                if state.dragging {
                    state.dragging = false;
                    shell.request_redraw();
                } else if !state.modifiers.control() {
                    // A plain click on empty space: nothing selected.
                    shell.publish((self.on_change)(Change::Started { additive: false }));
                    shell.publish((self.on_change)(Change::Selected(Vec::new())));
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
        renderer: &Renderer,
    ) -> mouse::Interaction {
        if tree.state.downcast_ref::<State>().dragging {
            return mouse::Interaction::Crosshair;
        }
        layout
            .children()
            .next()
            .map_or(mouse::Interaction::None, |content| {
                self.content.as_widget().mouse_interaction(
                    &tree.children[0],
                    content,
                    cursor,
                    viewport,
                    renderer,
                )
            })
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
        if let Some(content) = layout.children().next() {
            self.content.as_widget().draw(
                &tree.children[0],
                renderer,
                theme,
                style,
                content,
                cursor,
                viewport,
            );
        }
        let state = tree.state.downcast_ref::<State>();
        let Some(start) = state.start.filter(|_| state.dragging) else {
            return;
        };
        let accent: Color = theme.cosmic().accent_color().into();
        renderer::Renderer::fill_quad(
            renderer,
            renderer::Quad {
                bounds: rectangle(start, state.current),
                border: Border {
                    color: accent,
                    width: 1.0,
                    radius: 2.0.into(),
                },
                ..Default::default()
            },
            Color { a: 0.18, ..accent },
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
        let content = layout.children().next()?;
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            content,
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
        if let Some(content) = layout.children().next() {
            self.content.as_widget().drag_destinations(
                &state.children[0],
                content,
                renderer,
                dnd_rectangles,
            );
        }
    }
}

impl<'a, Message: 'a> From<Marquee<'a, Message>> for Element<'a, Message> {
    fn from(marquee: Marquee<'a, Message>) -> Self {
        Element::new(marquee)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rectangle_works_in_any_drag_direction() {
        let a = Point::new(10.0, 50.0);
        let b = Point::new(30.0, 20.0);
        assert_eq!(rectangle(a, b), rectangle(b, a));
        assert_eq!(
            rectangle(a, b),
            Rectangle {
                x: 10.0,
                y: 20.0,
                width: 20.0,
                height: 30.0
            }
        );
    }
}
