// node-bindings
#[macro_use] extern crate neon;
extern crate glutin;
extern crate gleam;
extern crate webrender;
extern crate app_units;
extern crate serde;
extern crate serde_json;
#[macro_use] extern crate serde_derive;
#[macro_use] extern crate log;
extern crate env_logger;

mod window;

use neon::prelude::*;
use window::{Window, GlyphInfo};

declare_types! {
    pub class JsWindow for Window {
        init(mut ctx) {
            let title = ctx.argument::<JsString>(0)?.value();
            let width = ctx.argument::<JsNumber>(1)?.value();
            let height = ctx.argument::<JsNumber>(2)?.value();

            let w = Window::new(title, width, height);

            Ok(w)
        }

        method createBucket(mut ctx) {
            let data = ctx.argument::<JsString>(0)?.value();
            let item = serde_json::from_str(&data).unwrap();

            let index = {
                let mut this = ctx.this();
                let guard = ctx.lock();
                let mut w = this.borrow_mut(&guard);

                w.create_bucket(item)
            };

            // TODO: maybe we can restrict vector size?
            Ok(ctx.number(index as f64).upcast())
        }

        method updateBucket(mut ctx) {
            let bucket = ctx.argument::<JsNumber>(0)?.value() as usize;

            let data = ctx.argument::<JsString>(1)?.value();
            let item = serde_json::from_str(&data).unwrap();

            let mut this = ctx.this();

            ctx.borrow_mut(&mut this, |mut w| w.update_bucket(bucket, item));

            Ok(ctx.undefined().upcast())
        }

        method render(mut ctx) {
            let data = ctx.argument::<JsString>(0)?.value();
            let request = serde_json::from_str(&data).unwrap();
            let mut this = ctx.this();

            ctx.borrow_mut(&mut this, |mut w| w.render(request));

            Ok(ctx.undefined().upcast())
        }

        // TODO: array buffer?
        method getGlyphInfos(mut ctx) {
            let str = ctx.argument::<JsString>(0)?.value();
            let mut this = ctx.this();

            let glyph_infos = ctx.borrow(&mut this, |w| w.get_glyph_infos(&str));

            let js_array = JsArray::new(&mut ctx, (glyph_infos.len() * 2) as u32);

            // flat buffer of index + advance pairs
            for (i, GlyphInfo(glyph_index, advance)) in glyph_infos.iter().enumerate() {
                let j = i * 2;

                let js_num = ctx.number(*glyph_index);
                let _ = js_array.set(&mut ctx, j as u32, js_num);

                let js_num = ctx.number(*advance);
                let _ = js_array.set(&mut ctx, (j + 1) as u32, js_num);
            }

            Ok(js_array.upcast())
        }
    }
}

register_module!(mut ctx, {
    env_logger::init();

    ctx.export_class::<JsWindow>("Window")
});
