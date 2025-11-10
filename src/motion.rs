use esp_idf_svc::hal::gpio::{PinDriver, Output, OutputPin, Pin};
use esp_idf_svc::hal::timer::TimerDriver;

type OutputPinDriver<'a, Pin> = PinDriver<'a, Pin, Output>;

enum Direction {
    Clockwise,
    CounterClockwise
}

// unify the control of two steppers
// pub struct CoreXY {
//     m1: Stepper,
//     m2: Stepper,
// }


// eventually this will be private. you just pass pins into CoreXY and it builds two Steppers
pub struct Stepper<'a, StepPin, DirPin, EnPin>
where
    StepPin: Pin,
    DirPin: Pin,
    EnPin: Pin,
{
    left_motor_pos: u32,
    right_motor_pos: u32,
    step: OutputPinDriver<'a, StepPin>,
    dir: OutputPinDriver<'a, DirPin>,
    en: OutputPinDriver<'a, EnPin>,
}

impl<'a, StepPin, DirPin, EnPin> Stepper<'a, StepPin, DirPin, EnPin>
where
    StepPin: OutputPin + Pin,
    DirPin: OutputPin + Pin,
    EnPin: OutputPin + Pin,
{
    pub fn new(
        step: OutputPinDriver<'a, StepPin>,
        dir: OutputPinDriver<'a, DirPin>,
        en: OutputPinDriver<'a, EnPin>,
    ) -> Self {
        Self {
            left_motor_pos: 0,
            right_motor_pos: 0,
            step, dir, en
        }
    }

    fn step_once(&mut self, res: u16, direction: Direction) {
        // todo: verify direction is actually this
        match direction {
            Direction::Clockwise => self.dir.set_high().unwrap(),
            Direction::CounterClockwise => self.dir.set_low().unwrap(),
        };

        // todo: is enable low or high when moving? 
        self.en.set_low().unwrap();




    }

    pub fn home(&mut self) {}
    pub fn get_pos(self) -> [u32; 2] { [self.left_motor_pos, self.right_motor_pos] }
    pub fn move_to(&mut self) {}
}
