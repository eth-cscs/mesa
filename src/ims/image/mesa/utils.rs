use crate::ims::image::r#struct::Image;

pub async fn filter(image_vec: &mut [Image]) {
    // Sort images by creation time order ASC
    image_vec.sort_by(|a, b| a.created.as_ref().unwrap().cmp(b.created.as_ref().unwrap()));
}
