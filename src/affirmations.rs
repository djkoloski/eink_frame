use inky::{Color, Inky};
use inky_graphics::{Alignment, Graphics};
use rand::Rng;

const AFFIRMATIONS: &[&str] = &[
    "rm -rf --no-preserve-root /",
    "[object Object]",
    "We all died in 2020 and this is Hell",
    "Daddy needs his iBeer",
    "Error: no route to AWS-US-EAST-1",
    "I fucking hate you and hope you die",
];

pub fn render(inky: &mut Inky, graphics: &Graphics) {
    // let message =
    // AFFIRMATIONS[rand::rng().random_range(0..AFFIRMATIONS.len())];
    // graphics.draw_text(
    //     inky,
    //     650,
    //     100,
    //     &message,
    //     Alignment::Center,
    //     "helvB12",
    //     Color::Black,
    // );
}
