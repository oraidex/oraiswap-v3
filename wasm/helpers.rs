use wasm_bindgen::prelude::*;

// Logging to typescript
#[wasm_bindgen]
extern "C" {
    // Use `js_namespace` here to bind `console.log(..)` instead of just
    // `log(..)`
    #[wasm_bindgen(js_namespace = console)]
    pub fn log(s: &str);

    // The `console.log` is quite polymorphic, so we can bind it with multiple
    // signatures. Note that we need to use `js_name` to ensure we always call
    // `log` in JS.
    #[wasm_bindgen(js_namespace = console, js_name = log)]
    pub fn log_u32(a: u32);

    // Multiple arguments too!
    #[wasm_bindgen(js_namespace = console, js_name = log)]
    pub fn log_many(a: &str, b: &str);
}

#[macro_export]
macro_rules! decimal_ops {
    ($decimal:ident) => {
        ::paste::paste! {
            #[wasm_bindgen]
            pub fn [<get $decimal Scale >] () -> BigInt {
                BigInt::from($decimal::scale())
            }

            #[wasm_bindgen]
            pub fn [<get $decimal Denominator >] () -> BigInt {
                BigInt::from($decimal::from_integer(1).get())
            }

            #[wasm_bindgen]
            pub fn [<to $decimal >] (integer: u64, scale: Option<u8>) -> BigInt {
                BigInt::from($decimal::from_scale(integer, scale.unwrap_or_else(|| $decimal::scale())).get())
            }
        }
    };
}

#[macro_export]
macro_rules! vec_adapter {
    ($item:ident) => {
        ::paste::paste! {
            #[derive(Clone, Debug, PartialEq, derive_more::Deref, derive_more::DerefMut, Serialize, Deserialize, tsify::Tsify)]
            #[tsify(into_wasm_abi, from_wasm_abi)]
            pub struct [<$item Vec>](#[tsify(type = "" $item "[]")] Vec<$item>);
        }
    };
}
