use esp_idf_svc::hal::delay::Delay;
use esp_idf_svc::hal::gpio::{Input, Output, Pin, PinDriver};
use esp_idf_svc::hal::timer::TimerDriver;

type OutputPinDriver<'a, Pin> = PinDriver<'a, Pin, Output>;
type InputPinDriver<'a, Pin> = PinDriver<'a, Pin, Input>;
// 1.25" squares
const SQUARE_SIZE_MM: f32 = 31.75;

// MS1/MS2 jumper settings for the TMC2209 (assumes MS3/SPREAD are low).
// spread low = stealthChop | high -> spreadCycle
// these will be hardcoded (no need to use additional gpio ports) 
// but this needs to be synced with the actual ms1/ms2
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Microstep {
    //      ms1/2
    Eight = 8, // low, low 
    Sixteen = 16, // high, high
    ThirtyTwo = 32, // high, low
    SixtyFour = 64, // low, high
}

impl Microstep {
    pub const fn steps_per_revolution(self, motor_steps_per_rev: u32) -> u32 {
        motor_steps_per_rev * self as u32
    }
}

/// Sequence of squares plus magnet toggle events for a move.
pub struct MoveInstruction {
    coords: Vec<u8>,  // 0-63 top left to bottom right
    drags: Vec<bool>, // true = engage magnet
}

impl MoveInstruction {
    pub fn new(coords: Vec<u8>, drags: Vec<bool>) -> Self {
        assert_eq!(coords.len(), drags.len());
        Self { coords, drags }
    }

    pub fn iter(&self) -> impl Iterator<Item = (u8, bool)> + '_ {
        self.coords.iter().copied().zip(self.drags.iter().copied())
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Clockwise,
    CounterClockwise,
}
/// CoreXY gantry; +X means both steppers spin the same way, +Y spins them opposite.
pub struct CoreXY<'a, LStepper, RStepper, MagnetPin, LeftLimitPin, RightLimitPin>
where
    LStepper: StepperOps,
    RStepper: StepperOps,
    MagnetPin: Pin,
    LeftLimitPin: Pin,
    RightLimitPin: Pin,
{
    left: LStepper,
    right: RStepper,
    magnet: OutputPinDriver<'a, MagnetPin>,
    left_limit: InputPinDriver<'a, LeftLimitPin>,
    right_limit: InputPinDriver<'a, RightLimitPin>,
    timer: TimerDriver<'a>,
    microstep: Microstep,
}

impl<'a, LStepper, RStepper, MagnetPin, LeftLimitPin, RightLimitPin>
    CoreXY<'a, LStepper, RStepper, MagnetPin, LeftLimitPin, RightLimitPin>
where
    LStepper: StepperOps,
    RStepper: StepperOps,
    MagnetPin: Pin,
    LeftLimitPin: Pin,
    RightLimitPin: Pin,
{
    const MOTOR_STEPS_PER_REV: u32 = 200; // 1.8° per full step
    const BELT_PITCH_MM: f32 = 2.0; // GT2 belt pitch
    const PULLEY_TEETH: f32 = 20.0;
    const MILLIMETERS_PER_REV: f32 = Self::BELT_PITCH_MM * Self::PULLEY_TEETH;
    const HOMING_MAX_STEPS: u32 = 80_000;
    const HOMING_BACKOFF_STEPS: u32 = 1_000;

    fn steps_per_mm(&self) -> f32 {
        let steps_per_rev = self
            .microstep
            .steps_per_revolution(Self::MOTOR_STEPS_PER_REV) as f32;
        steps_per_rev / Self::MILLIMETERS_PER_REV
    }

    fn ticks_per_step(&self) -> u64 {
        self.timer.tick_hz() / 2000 as u64
    }

    fn delay_ticks(&mut self, ticks: u64) {
        esp_idf_svc::hal::task::block_on(self.timer.delay(ticks)).unwrap();
    }

    fn step_pair(&mut self, ticks_per_step: u64) {
        self.left.step_once();
        self.right.step_once();
        self.delay_ticks(ticks_per_step);
    }

    fn drive_until<F>(&mut self, dir_left: Direction, dir_right: Direction, mut condition: F, max_steps: u32)
    where
        F: Fn(&Self) -> bool,
    {
        self.left.set_direction(dir_left);
        self.right.set_direction(dir_right);
        let ticks_per_step = self.ticks_per_step();
        let mut steps = 0;
        while !condition(self) {
            if steps >= max_steps {
                panic!("CoreXY homing did not reach expected switch state");
            }
            self.step_pair(ticks_per_step);
            steps += 1;
        }
    }

    fn direction_and_steps(delta_steps: f32) -> (Direction, i32) {
        let steps = delta_steps.round() as i32;
        let dir = if steps >= 0 {
            Direction::Clockwise
        } else {
            Direction::CounterClockwise
        };
        (dir, steps.abs())
    }

    pub fn new(
        left: LStepper,
        right: RStepper,
        magnet: OutputPinDriver<'a, MagnetPin>,
        left_limit: InputPinDriver<'a, LeftLimitPin>,
        right_limit: InputPinDriver<'a, RightLimitPin>,
        mut timer: TimerDriver<'a>,
        microstep: Microstep,
    ) -> Self {
        timer.enable(true).unwrap();
        let mut core = Self {
            left,
            right,
            magnet,
            left_limit,
            right_limit,
            timer,
            microstep,
        };
        core.set_magnet(false);
        // todo!(): re-enable homing once limit switches are verified
        // core.home();
        core
    }

    fn set_magnet(&mut self, engage: bool) {
        if engage {
            self.magnet.set_high()
        } else {
            self.magnet.set_low()
        }
        .ok();
    }

    pub fn goto(&mut self, square: u32) {
        // left motor, right motor
        let steps_per_mm = self.steps_per_mm();
        let (left_steps, right_steps) = (
            self.left.position() as f32,
            self.right.position() as f32,
        );
        let (left_mm, right_mm) = (
            (left_steps + right_steps) / (2.0 * steps_per_mm),
            (left_steps - right_steps) / (2.0 * steps_per_mm),
        );

        let target = (
            ((square / 8) as f32 + 0.5) * SQUARE_SIZE_MM,
            ((square % 8) as f32 + 0.5) * SQUARE_SIZE_MM,
        );
        let (dx, dy) = (target.0 - left_mm, target.1 - right_mm);
        let (dir_a, steps_a) = Self::direction_and_steps((dx + dy) * steps_per_mm);
        let (dir_b, steps_b) = Self::direction_and_steps((dx - dy) * steps_per_mm);

        self.left.set_direction(dir_a);
        self.right.set_direction(dir_b);

        let total = steps_a.max(steps_b).max(1);
        let mut acc_a = 0;
        let mut acc_b = 0;

        let ticks_per_step = self.ticks_per_step(); // todo! tune this

        for _ in 0..total {
            acc_a += steps_a;
            acc_b += steps_b;

            if acc_a >= total {
                self.left.step_once();
                acc_a -= total;
            }
            if acc_b >= total {
                self.right.step_once();
                acc_b -= total;
            }

            self.delay_ticks(ticks_per_step);
        }
    }

    pub fn consume_instructions(&mut self, instructions: MoveInstruction) {
        for (sq, magnet) in instructions.iter() {
            self.goto(sq as u32);
            self.set_magnet(magnet);
        }
    }

    pub fn home(&mut self) {
        // Move toward bottom-left along -X until X switch engages.
        self.drive_until(
            Direction::CounterClockwise,
            Direction::CounterClockwise,
            |s| s.left_limit.is_low(),
            Self::HOMING_MAX_STEPS,
        );

        // Back off +X just until the switch releases.
        self.drive_until(
            Direction::Clockwise,
            Direction::Clockwise,
            |s| !s.left_limit.is_low(),
            Self::HOMING_BACKOFF_STEPS,
        );

        // Move toward bottom-left along -Y (motors opposite) until Y switch engages.
        self.drive_until(
            Direction::CounterClockwise,
            Direction::Clockwise,
            |s| s.right_limit.is_low(),
            Self::HOMING_MAX_STEPS,
        );

        // Back off +Y to clear the switch.
        self.drive_until(
            Direction::Clockwise,
            Direction::CounterClockwise,
            |s| !s.right_limit.is_low(),
            Self::HOMING_BACKOFF_STEPS,
        );

        self.left.reset_position();
        self.right.reset_position();
    }
}

pub trait StepperOps {
    fn set_direction(&mut self, direction: Direction);
    fn step_once(&mut self);
    fn position(&self) -> i32;
    fn reset_position(&mut self);
}

pub struct Stepper<'a, StepPin, DirPin, EnPin>
where
    StepPin: Pin,
    DirPin: Pin,
    EnPin: Pin,
{
    pos: i32,
    step: OutputPinDriver<'a, StepPin>,
    dir: OutputPinDriver<'a, DirPin>,
    en: OutputPinDriver<'a, EnPin>,
    current_direction: Direction,
}

impl<'a, StepPin, DirPin, EnPin> Stepper<'a, StepPin, DirPin, EnPin>
where
    StepPin: Pin,
    DirPin: Pin,
    EnPin: Pin,
{
    pub fn new(
        step: OutputPinDriver<'a, StepPin>,
        dir: OutputPinDriver<'a, DirPin>,
        en: OutputPinDriver<'a, EnPin>,
    ) -> Self {
        let mut stepper = Self {
            pos: 0,
            step,
            dir,
            en,
            current_direction: Direction::Clockwise,
        };
        stepper.set_direction(Direction::Clockwise);
        stepper
    }
}

impl<'a, StepPin, DirPin, EnPin> StepperOps for Stepper<'a, StepPin, DirPin, EnPin>
where
    StepPin: Pin,
    DirPin: Pin,
    EnPin: Pin,
{
    fn set_direction(&mut self, direction: Direction) {
        match direction {
            Direction::Clockwise => {
                self.dir.set_high().ok();
                self.current_direction = Direction::Clockwise
            }
            Direction::CounterClockwise => {
                self.dir.set_low().ok();
                self.current_direction = Direction::CounterClockwise
            }
        };
    }

    fn step_once(&mut self) {
        self.step.set_high().ok(); 
        let delay = Delay::new_default();
        /* 
        the delay is already higher than 1us due to function calls, but let's make it explicit. 
         */
        delay.delay_us(1);
        self.step.set_low().ok();

        if self.current_direction == Direction::Clockwise {
            self.pos += 1;
        } else {
            self.pos -= 1; 
        }
    }

    fn position(&self) -> i32 { self.pos }

    fn reset_position(&mut self) { self.pos = 0; }
}
