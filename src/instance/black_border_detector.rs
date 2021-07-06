use crate::{image::Image, models};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlackBorder {
    pub unknown: bool,
    pub horizontal_size: u32,
    pub vertical_size: u32,
    threshold: u8,
}

impl BlackBorder {
    pub fn new(threshold: u8) -> Self {
        Self {
            unknown: true,
            horizontal_size: 0,
            vertical_size: 0,
            threshold,
        }
    }

    fn is_black(&self, color: models::Color) -> bool {
        color.red < self.threshold && color.green < self.threshold && color.blue < self.threshold
    }

    fn update(&mut self, xy: (Option<u32>, Option<u32>)) {
        if let (Some(x), Some(y)) = xy {
            self.unknown = false;
            self.horizontal_size = y;
            self.vertical_size = x;
        } else {
            self.unknown = true;
            self.horizontal_size = 0;
            self.vertical_size = 0;
        }
    }

    fn process_default(&mut self, image: &impl Image) {
        let width = image.width();
        let height = image.height();
        let width33 = width / 3;
        let height33 = height / 3;
        let width66 = width33 * 2;
        let height66 = height33 * 2;
        let x_center = width / 2;
        let y_center = height / 2;

        let width = width - 1;
        let height = height - 1;

        // Safety: width33 < width && height33 < height so x and y are in range
        unsafe {
            let first_non_black_x = (0..width33).find(|x| {
                !self.is_black(image.color_at_unchecked(width - *x, y_center))
                    || !self.is_black(image.color_at_unchecked(*x, height33))
                    || !self.is_black(image.color_at_unchecked(*x, height66))
            });

            let first_non_black_y = (0..height33).find(|y| {
                !self.is_black(image.color_at_unchecked(x_center, height - *y))
                    || !self.is_black(image.color_at_unchecked(width33, *y))
                    || !self.is_black(image.color_at_unchecked(width66, *y))
            });

            self.update((first_non_black_x, first_non_black_y));
        }
    }

    fn process_classic(&mut self, image: &impl Image) {
        let width = image.width() / 3;
        let height = image.height() / 3;
        let max_size = width.max(height);

        let mut first_non_black_x = -1i32;
        let mut first_non_black_y = -1i32;

        for i in 0..max_size {
            let x = i.min(width);
            let y = i.min(height);

            // Safety: x and y are in range, since width < image.width()
            unsafe {
                if !self.is_black(image.color_at_unchecked(x, y)) {
                    first_non_black_x = x as _;
                    first_non_black_y = y as _;
                }
            }
        }

        while first_non_black_x > 0 {
            // Safety: first_non_black_x > 0 && first_non_black_x <= width
            unsafe {
                if first_non_black_y < 0
                    || self.is_black(
                        image.color_at_unchecked(
                            (first_non_black_x - 1) as _,
                            first_non_black_y as _,
                        ),
                    )
                {
                    break;
                }
            }

            first_non_black_x -= 1;
        }

        while first_non_black_y > 0 {
            // Safety: first_non_black_x >= 0 && first_non_black_y > 0
            unsafe {
                if self.is_black(
                    image.color_at_unchecked(first_non_black_x as _, (first_non_black_y - 1) as _),
                ) {
                    break;
                }
            }

            first_non_black_y -= 1;
        }

        self.update((
            if first_non_black_x < 0 {
                None
            } else {
                Some(first_non_black_x as u32)
            },
            if first_non_black_y < 0 {
                None
            } else {
                Some(first_non_black_y as u32)
            },
        ));
    }

    fn process_osd(&mut self, image: &impl Image) {
        let width = image.width();
        let height = image.height();
        let width33 = width / 3;
        let height33 = height / 3;
        let height66 = height33 * 2;
        let y_center = height / 2;

        let width = width - 1;
        let height = height - 1;

        // Safety: all operations are in range of the image dimensions
        unsafe {
            let first_non_black_x = (0..width33).find(|x| {
                !self.is_black(image.color_at_unchecked(width - *x, y_center))
                    || !self.is_black(image.color_at_unchecked(*x, height33))
                    || !self.is_black(image.color_at_unchecked(*x, height66))
            });

            let x = first_non_black_x.unwrap_or(width33);

            let first_non_black_y = (0..height33).find(|y| {
                !self.is_black(image.color_at_unchecked(x, *y))
                    || !self.is_black(image.color_at_unchecked(x, height - *y))
                    || !self.is_black(image.color_at_unchecked(width - x, *y))
                    || !self.is_black(image.color_at_unchecked(width - x, height - *y))
            });

            self.update((first_non_black_x, first_non_black_y));
        }
    }

    fn process_letterbox(&mut self, image: &impl Image) {
        let width = image.width();
        let height = image.height();
        let height33 = height / 3;
        let width25 = width / 4;
        let width75 = width25 * 3;
        let x_center = width / 2;

        let height = height - 1;

        // Safety: all operations are in range of the image dimensions
        unsafe {
            let first_non_black_y = (0..height33).find(|y| {
                !self.is_black(image.color_at_unchecked(x_center, *y))
                    || !self.is_black(image.color_at_unchecked(width25, *y))
                    || !self.is_black(image.color_at_unchecked(width75, *y))
                    || !self.is_black(image.color_at_unchecked(width25, height - *y))
                    || !self.is_black(image.color_at_unchecked(width75, height - *y))
            });

            self.update((
                first_non_black_y,
                if first_non_black_y.is_none() {
                    None
                } else {
                    Some(0)
                },
            ));
        }
    }

    pub fn process(&mut self, image: &impl Image, mode: models::BlackBorderDetectorMode) {
        match mode {
            models::BlackBorderDetectorMode::Default => self.process_default(image),
            models::BlackBorderDetectorMode::Classic => self.process_classic(image),
            models::BlackBorderDetectorMode::Osd => self.process_osd(image),
            models::BlackBorderDetectorMode::Letterbox => self.process_letterbox(image),
        }
    }

    pub fn blur(&mut self, blur: u32) {
        if self.horizontal_size > 0 {
            self.horizontal_size += blur;
        }

        if self.vertical_size > 0 {
            self.vertical_size += blur;
        }
    }

    pub fn get_ranges(
        &self,
        width: u32,
        height: u32,
    ) -> (std::ops::Range<u32>, std::ops::Range<u32>) {
        if self.unknown {
            (0..width, 0..height)
        } else {
            (
                self.vertical_size.min(width / 2)..(width - self.vertical_size).max(width / 2),
                self.horizontal_size.min(height / 2)
                    ..(height - self.horizontal_size).max(height / 2),
            )
        }
    }
}

impl Default for BlackBorder {
    fn default() -> Self {
        Self {
            unknown: true,
            horizontal_size: 0,
            vertical_size: 0,
            threshold: 0,
        }
    }
}

pub struct BlackBorderDetector {
    config: models::BlackBorderDetector,
    current_border: BlackBorder,
    previous_border: BlackBorder,
    consistent_cnt: u32,
    inconsistent_cnt: u32,
}

impl BlackBorderDetector {
    pub fn new(config: models::BlackBorderDetector) -> Self {
        Self {
            config,
            current_border: Default::default(),
            previous_border: Default::default(),
            consistent_cnt: 0,
            inconsistent_cnt: 0,
        }
    }

    fn threshold(&self) -> u8 {
        (self.config.threshold * 255 / 100).min(255).max(0) as u8
    }

    fn update_border(&mut self, new_border: BlackBorder) -> bool {
        if new_border == self.previous_border {
            self.consistent_cnt += 1;
            self.inconsistent_cnt = 0;
        } else {
            self.inconsistent_cnt += 1;

            if self.inconsistent_cnt <= self.config.max_inconsistent_cnt {
                return false;
            }

            self.previous_border = new_border;
            self.consistent_cnt = 0;
        }

        if self.current_border == new_border {
            self.inconsistent_cnt = 0;
            return false;
        }

        if new_border.unknown {
            if self.consistent_cnt == self.config.unknown_frame_cnt {
                self.current_border = new_border;
                return true;
            }
        } else {
            if self.current_border.unknown || self.consistent_cnt == self.config.border_frame_cnt {
                self.current_border = new_border;
                return true;
            }
        }

        false
    }

    pub fn current_border(&self) -> BlackBorder {
        self.current_border
    }

    /// Process the given image
    ///
    /// # Returns
    ///
    /// true if a different border was detected, false otherwise
    pub fn process(&mut self, image: &impl Image) -> bool {
        let mut image_border = BlackBorder::new(self.threshold());

        if !self.config.enable {
            return self.update_border(image_border);
        }

        image_border.process(image, self.config.mode);
        image_border.blur(self.config.blur_remove_cnt);

        self.update_border(image_border)
    }
}
