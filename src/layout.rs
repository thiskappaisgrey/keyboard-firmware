use keyberon::action::{k, l, HoldTapAction, HoldTapConfig};
use keyberon::key_code::KeyCode::*;

pub type Action = keyberon::action::Action<()>;

pub static LAYERS: keyberon::layout::Layers<7, 5, 1, Action> = [[
    [k(Q), k(Q), k(Q), k(Q), k(Q), k(Q), k(Q)],
    [k(Q), k(Q), k(Q), k(Q), k(Q), k(Q), k(Q)],
    [k(Q), k(Q), k(Q), k(Q), k(Q), k(Q), k(Q)],
    [k(Q), k(Q), k(Q), k(Q), k(Q), k(Q), k(Q)],
    [k(Q), k(Q), k(Q), k(Q), k(Q), k(Q), k(Q)],
]];
