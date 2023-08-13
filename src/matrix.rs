use keyberon::matrix::Matrix;
use sparkfun_pro_micro_rp2040::{hal::gpio::DynPin, Pins};
// use rp2040_hal::gpio::Pin;
// matrix definitions for the pinky4

// refer to this: https://cdn.sparkfun.com/assets/e/2/7/6/b/ProMicroRP2040_Graphical_Datasheet.pdf
// for the pin definitions

// the corresponding pins is on the pcb.
pub type Pinky4Matrix = Matrix<DynPin, DynPin, 7, 5>;
// TODO the last row has some missing keys ..
// this is because of how the type definition of the matrix is defined..
pub fn init_matrix(pins: &mut Pins) -> Pinky4Matrix {
    // apparently the colums are a pull_up_input
    // but the rows are a push_pull_output? not sure why
    Matrix::new(
        [
            pins.adc3.into_pull_up_input().into(),
            pins.adc2.into_pull_up_input().into(),
            pins.adc1.into_pull_up_input().into(),
            pins.adc0.into_pull_up_input().into(),
            pins.sck.into_pull_up_input().into(),
            pins.cipo.into_pull_up_input().into(),
            pins.copi.into_pull_up_input().into(),
        ],
        [
            pins.gpio4.into_push_pull_output().into(),
            pins.gpio5.into_push_pull_output().into(),
            pins.gpio6.into_push_pull_output().into(),
            pins.gpio7.into_push_pull_output().into(),
            pins.tx1.into_push_pull_output().into(),
        ],
    )
    .unwrap()
}
