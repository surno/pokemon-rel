use image::DynamicImage;

pub enum Frame {
    Ping,
    Handshake {
        version: u32,
        name: String,
        program: u16,
    },
    Image {
        image: DynamicImage,
    },
    ImageGD2 {
        width: u32,
        height: u32,
        gd2_data: Vec<u8>,
    },
    Shutdown,
}
