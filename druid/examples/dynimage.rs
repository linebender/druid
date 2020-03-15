use druid::{
    widget::{FillStrat, Flex, Image, ImageData, WidgetExt, },
    AppLauncher, Widget, WindowDesc,
};
use image::{DynamicImage, GenericImage, Rgba};

fn ui_builder() -> impl Widget<u8> {
    let png_data = ImageData::from_file("examples/PicWithAlpha.png").unwrap();
    let mut dyn_img = DynamicImage::new_rgba8(300, 300);
    let color = Rgba([155, 133, 100, 125]);
    unsafe {
        for i in 0..280 {
            for j in 0..20 {
                dyn_img.unsafe_put_pixel(i, i+j, color.clone());
            }
            dyn_img.unsafe_put_pixel(i, 300 - i - 1, color);
        }
    }
    let dyn_data = ImageData::from_dynamic_image(dyn_img);
    let img0 = Image::new(png_data.clone()).fill_mode(FillStrat::ScaleDown).fix_width(500.).center();
    let img1 = Image::new(dyn_data).fill_mode(FillStrat::ScaleDown).fix_width(300.).center();
    let root = Flex::column()
        .with_child(img0, 1.)
        .with_child(img1, 1.);
    root
}

#[cfg(feature = "image")]
fn main() {
    let main_window = WindowDesc::new(ui_builder);
    let data = 0_u8;
    AppLauncher::with_window(main_window)
        .use_simple_logger()
        .launch(data)
        .expect("launch failed");
}


