use crate::css::{matching_rules, Style, StyleProp, StyleSheet};
use crate::gfx::{Frame, Text, TextStyle, Vec2, AABB};
use crate::layout::LayoutNode;
use crate::renderer::Renderer;
use crate::util::SlotMap;
use crate::{Document, DocumentEvent, NodeId, NodeType};
use std::cell::RefCell;
use std::rc::Rc;

pub struct Viewport {
    size: (i32, i32),

    document: Rc<RefCell<Document>>,
    styles: Rc<RefCell<SlotMap<NodeId, Style>>>,
    layout_nodes: Rc<RefCell<SlotMap<NodeId, LayoutNode>>>,
    texts: Rc<RefCell<SlotMap<NodeId, Text>>>,
    renderer: Renderer,
}

impl Viewport {
    pub fn new(size: (i32, i32), document: &Rc<RefCell<Document>>) -> Self {
        let styles = Rc::new(RefCell::new(SlotMap::new()));
        let layout_nodes = Rc::new(RefCell::new(SlotMap::new()));
        let texts = Rc::new(RefCell::new(SlotMap::new()));
        let renderer = Renderer::new(document, &layout_nodes, &styles, &texts);

        // create root layout node
        layout_nodes
            .borrow_mut()
            .put(document.borrow().root(), LayoutNode::new());
        update_layout_node(
            &mut layout_nodes.borrow_mut()[0],
            &Style::from("display: block; width: 100%; height: 100%"),
        );

        let viewport = Self {
            size,
            document: Rc::clone(&document),
            layout_nodes: Rc::clone(&layout_nodes),
            styles: Rc::clone(&styles),
            texts: Rc::clone(&texts),
            renderer,
        };

        document.borrow_mut().add_listener(move |doc, e| {
            use DocumentEvent::*;

            match *e {
                Create(node, NodeType::Element) => {
                    layout_nodes.borrow_mut().put(node, LayoutNode::new());
                    styles.borrow_mut().put(node, Style::new());
                }
                Create(node, NodeType::Text) => {
                    let texts2 = Rc::clone(&texts);
                    let measure = move |max_width| texts2.borrow()[node].measure(max_width);

                    texts
                        .borrow_mut()
                        .put(node, Text::new(doc.cdata(node), &TextStyle::DEFAULT));
                    layout_nodes.borrow_mut().put(node, LayoutNode::new_leaf(measure));
                }
                Create(node, NodeType::Comment) => {
                    layout_nodes.borrow_mut().put(node, LayoutNode::new());
                    update_layout_node(&mut layout_nodes.borrow_mut()[node], &Style::HIDDEN);
                }

                Insert(parent, child, index) => {
                    layout_nodes.borrow()[parent].insert_child(&layout_nodes.borrow()[child], index)
                }
                Remove(parent, child) => layout_nodes.borrow()[parent].remove_child(&layout_nodes.borrow()[child]),

                Cdata(node, cdata) => {
                    texts.borrow_mut()[node].set_text(cdata);
                    layout_nodes.borrow()[node].mark_dirty();
                }

                Drop(node) => {
                    layout_nodes.borrow_mut().remove(node);
                    styles.borrow_mut().remove(node);
                }

                _ => {}
            }
        });

        viewport
    }

    pub fn document(&self) -> &Rc<RefCell<Document>> {
        &self.document
    }

    pub fn render(&mut self) -> Frame {
        self.update();

        self.renderer.render()
    }

    pub fn node_rect(&self, node: NodeId) -> (f32, f32, f32, f32) {
        let ln = &self.layout_nodes.borrow()[node];
        let (x, y) = ln.offset();
        let (w, h) = ln.size();

        (x, y, w, h)
    }

    // TODO: this is wrong anyway because children can be bigger than their parents
    //       so we will need some kind of AABB tree anyway
    // TODO: display: none
    // TODO: scroll
    // TODO: clip (hidden and/or radius)
    pub fn element_from_point(&self, point: (f32, f32)) -> Option<NodeId> {
        let pos = Vec2::from(point);

        let document = self.document.borrow();
        let layout_nodes = self.layout_nodes.borrow();

        let mut parent = document.first_child(document.root())?;
        let mut offset = Vec2::ZERO;
        let mut continue_down;

        // go down (starting from root) through each matching surface and return the last & deepest one
        loop {
            continue_down = false;
            offset = Vec2::from(layout_nodes[parent].offset()) + offset;

            for el in document.children(parent) {
                let rect = AABB::new(Vec2::ZERO, Vec2::from(layout_nodes[el].size()))
                    + Vec2::from(layout_nodes[el].offset())
                    + offset;

                if rect.contains(pos) {
                    parent = el;
                    continue_down = true;
                }
            }

            if !continue_down {
                return Some(parent);
            }
        }
    }

    // TODO: caretPositionFromPoint

    // TODO: getClientRect, offsetLeft, offsetTop, offsetWidth, offsetHeight

    // TODO: scrollTo(), scrollTop, ...

    // TODO: computed_style?

    pub fn resize(&mut self, size: (i32, i32)) {
        self.size = size;
        self.update();
    }

    fn update(&self) {
        self.update_styles();
        self.update_layout();
    }

    fn update_styles(&self) {
        let doc = self.document.borrow();
        let mut styles = self.styles.borrow_mut();
        let mut layout_nodes = self.layout_nodes.borrow_mut();

        let mut sheets: Vec<_> = doc
            .query_selector_all(doc.root(), "html > head > style")
            .iter()
            //.inspect(|s| println!("style: {}", doc.text_content(**s)))
            .map(|s| StyleSheet::from(&*doc.text_content(*s)))
            .collect();

        sheets.insert(0, StyleSheet::from(include_str!("../resources/ua.css")));

        doc.with_matching_context(|ctx| {
            for (el, out) in styles.iter_mut() {
                // TODO: just iterate props, no need to merge anymore
                let mut style = Style::new();

                for r in matching_rules(&ctx, &sheets, el) {
                    for p in r.style().props() {
                        style.add_prop(p.clone());
                    }
                }

                // add inline style
                for p in doc.element_style(el).props() {
                    style.add_prop(p.clone());
                }

                update_layout_node(&mut layout_nodes[el], &style);

                // TODO: keep just renderstyle
                *out = style;
            }
        });
    }

    fn update_layout(&self) {
        let size = (self.size.0 as _, self.size.1 as _);

        self.layout_nodes.borrow()[self.document.borrow().root()].calculate(size);
    }
}

fn update_layout_node(ln: &mut LayoutNode, style: &Style) {
    //println!("{}", style.css_text());

    use super::css::*;
    use super::layout::*;

    fn dim(d: &CssDimension) -> Dimension {
        match d {
            CssDimension::Px(v) => Dimension::Px(*v),
            CssDimension::Percent(v) => Dimension::Percent(*v),
            CssDimension::Auto => Dimension::Auto,
            //_ => Dimension::Undefined,
        }
    }

    fn align(d: &CssAlign) -> Align {
        match d {
            CssAlign::Auto => Align::Auto,
            CssAlign::FlexStart => Align::FlexStart,
            CssAlign::Center => Align::Center,
            CssAlign::FlexEnd => Align::FlexEnd,
            CssAlign::Stretch => Align::Stretch,
            CssAlign::Baseline => Align::Baseline,
            CssAlign::SpaceBetween => Align::SpaceBetween,
            CssAlign::SpaceAround => Align::SpaceAround,
        }
    }

    fn justify(d: &CssJustify) -> Justify {
        match d {
            CssJustify::FlexStart => Justify::FlexStart,
            CssJustify::Center => Justify::Center,
            CssJustify::FlexEnd => Justify::FlexEnd,
            CssJustify::SpaceBetween => Justify::SpaceBetween,
            CssJustify::SpaceAround => Justify::SpaceAround,
            CssJustify::SpaceEvenly => Justify::SpaceEvenly,
        }
    }

    for p in style.props() {
        use StyleProp as P;

        match p {
            P::Width(v) => ln.set_width(dim(v)),
            P::Height(v) => ln.set_height(dim(v)),
            P::MinWidth(v) => ln.set_min_width(dim(v)),
            P::MinHeight(v) => ln.set_min_height(dim(v)),
            P::MaxWidth(v) => ln.set_max_width(dim(v)),
            P::MaxHeight(v) => ln.set_max_height(dim(v)),

            P::PaddingTop(v) => ln.set_padding_top(dim(v)),
            P::PaddingRight(v) => ln.set_padding_right(dim(v)),
            P::PaddingBottom(v) => ln.set_padding_bottom(dim(v)),
            P::PaddingLeft(v) => ln.set_padding_left(dim(v)),

            P::MarginTop(v) => ln.set_margin_top(dim(v)),
            P::MarginRight(v) => ln.set_margin_right(dim(v)),
            P::MarginBottom(v) => ln.set_margin_bottom(dim(v)),
            P::MarginLeft(v) => ln.set_margin_left(dim(v)),

            P::Position(v) => ln.set_position(match v {
                CssPosition::Absolute => Position::Absolute,
                _ => Position::Relative,
            }),
            P::Top(v) => ln.set_top(dim(v)),
            P::Right(v) => ln.set_right(dim(v)),
            P::Bottom(v) => ln.set_bottom(dim(v)),
            P::Left(v) => ln.set_left(dim(v)),

            P::Display(v) => ln.set_display(match v {
                CssDisplay::None => Display::None,
                CssDisplay::Flex => Display::Flex,
                // TODO
                CssDisplay::Block => {
                    // weird but correct, what's missing is that all inlines should
                    // be wrapped in anonymous box/line, then it should work fine
                    // (but that "line" is not just flex-wrap: wrap)
                    ln.set_flex_direction(FlexDirection::Column);
                    ln.set_align_items(Align::Stretch);
                    Display::Flex
                }
                // TODO
                CssDisplay::Inline => {
                    //ln.set_flex_direction(FlexDirection::Row);
                    //ln.set_flex_wrap(FlexWrap::Wrap);
                    Display::Flex
                }
            }),
            P::FlexGrow(v) => ln.set_flex_grow(*v),
            P::FlexShrink(v) => ln.set_flex_shrink(*v),
            P::FlexBasis(v) => ln.set_flex_basis(dim(v)),
            P::FlexWrap(v) => ln.set_flex_wrap(match v {
                CssFlexWrap::NoWrap => FlexWrap::NoWrap,
                CssFlexWrap::Wrap => FlexWrap::Wrap,
                CssFlexWrap::WrapReverse => FlexWrap::WrapReverse,
            }),
            P::FlexDirection(v) => ln.set_flex_direction(match v {
                CssFlexDirection::Row => FlexDirection::Row,
                CssFlexDirection::Column => FlexDirection::Column,
                CssFlexDirection::RowReverse => FlexDirection::RowReverse,
                CssFlexDirection::ColumnReverse => FlexDirection::ColumnReverse,
            }),
            P::AlignContent(v) => ln.set_align_content(align(v)),
            P::AlignItems(v) => ln.set_align_items(align(v)),
            P::AlignSelf(v) => ln.set_align_self(align(v)),
            P::JustifyContent(v) => ln.set_justify_content(justify(v)),
            _ => {}
        }
    }
}
