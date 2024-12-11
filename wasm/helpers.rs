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
macro_rules! convert {
    ($value:expr) => {{
        serde_wasm_bindgen::from_value($value)
    }};
}

#[macro_export]
macro_rules! resolve {
    ($result:expr) => {{
        match $result {
            Ok(value) => Ok(serde_wasm_bindgen::to_value(&value)?),
            Err(error) => Err(JsValue::from_str(&error.to_string())),
        }
    }};
}

#[macro_export]
macro_rules! decimal_ops_uint {
    ($decimal:ident) => {
        ::paste::paste! {
            #[wasm_bindgen]
            #[allow(non_snake_case)]
            pub fn [<get $decimal Scale >] () -> BigInt {
                BigInt::from($decimal::scale())
            }

            #[wasm_bindgen]
            #[allow(non_snake_case)]
            pub fn [<get $decimal Denominator >] () -> BigInt {
                // should be enough for current denominators
                BigInt::from($decimal::from_integer(1).get().as_u128())
            }

            #[wasm_bindgen]
            #[allow(non_snake_case)]
            pub fn [<_to $decimal >] (js_val: JsValue, js_scale: JsValue) -> BigInt {
                let js_val: u64 = crate::convert!(js_val).unwrap();
                let scale: u64 = crate::convert!(js_scale).unwrap();
                $decimal::from_scale(js_val, scale as u8)
                .get().0
                .iter().rev()
                .fold(BigInt::from(0), |acc, &x| (acc << BigInt::from(64)) | BigInt::from(x))
            }
        }
    };
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
                use std::str::FromStr;
                BigInt::from_str(&$decimal::from_integer(1).to_string()).unwrap()
            }

            #[wasm_bindgen]
            pub fn [<to $decimal >] (integer: u64, scale: Option<u8>) -> BigInt {
                use std::str::FromStr;
                BigInt::from_str(&$decimal::from_scale(integer, scale.unwrap_or_else(|| $decimal::scale())).to_string()).unwrap()
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
