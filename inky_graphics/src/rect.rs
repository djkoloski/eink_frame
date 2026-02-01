use core::array;

#[derive(Clone, Debug)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    pub fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn split_off_right(&mut self, width: i32) -> Self {
        self.width -= width;
        Self {
            x: self.x + self.width,
            y: self.y,
            width,
            height: self.height,
        }
    }

    pub fn split_frac_off_right(&mut self, frac: f32) -> Self {
        self.split_off_right((self.width as f32 * frac).round() as i32)
    }

    pub fn split_off_left(&mut self, width: i32) -> Self {
        let result = Self {
            x: self.x,
            y: self.y,
            width,
            height: self.height,
        };

        self.x += width;
        self.width -= width;

        result
    }

    pub fn split_frac_off_left(&mut self, frac: f32) -> Self {
        self.split_off_left((self.width as f32 * frac).round() as i32)
    }

    pub fn split_off_bottom(&mut self, height: i32) -> Self {
        self.height -= height;

        Self {
            x: self.x,
            y: self.y + self.height,
            width: self.width,
            height,
        }
    }

    pub fn split_frac_off_bottom(&mut self, frac: f32) -> Self {
        self.split_off_bottom((self.height as f32 * frac).round() as i32)
    }

    pub fn split_off_top(&mut self, height: i32) -> Self {
        let result = Self {
            x: self.x,
            y: self.y,
            width: self.width,
            height,
        };

        self.y += height;
        self.height -= height;

        result
    }

    pub fn split_frac_off_top(&mut self, frac: f32) -> Self {
        self.split_off_top((self.height as f32 * frac).round() as i32)
    }

    pub fn align_inner(
        self,
        width: i32,
        height: i32,
        horizontal_align: f32,
        vertical_align: f32,
    ) -> Self {
        Rect {
            x: self.x
                + ((self.width - width) as f32 * horizontal_align).round()
                    as i32,
            y: self.y
                + ((self.height - height) as f32 * vertical_align).round()
                    as i32,
            width,
            height,
        }
    }

    pub fn divide_grid<const X: usize, const Y: usize>(self) -> [[Self; X]; Y] {
        let w = X as i32;
        let h = Y as i32;

        array::from_fn(|y| {
            let y = y as i32;
            array::from_fn(|x| {
                let x = x as i32;
                Rect::new(
                    self.x + self.width * x / w,
                    self.y + self.height * y / h,
                    (self.width * (x + 1) / w) - (self.width * x / w),
                    (self.height * (y + 1) / h) - (self.height * y / h),
                )
            })
        })
    }

    pub fn shrink(self, right: i32, top: i32, left: i32, bottom: i32) -> Self {
        Self {
            x: self.x + left,
            y: self.y + top,
            width: self.width - right - left,
            height: self.height - top - bottom,
        }
    }
}
