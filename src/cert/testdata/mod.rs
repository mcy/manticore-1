// Copyright lowRISC contributors.
// Licensed under the Apache License, Version 2.0, see LICENSE for details.
// SPDX-License-Identifier: Apache-2.0

//! Generated certificate test data.
//!
//! Do not edit this file by hand; regenerate it with
//! `src/cert/testdata/generate.sh` instead. You'll need to install the
//! https://github.com/google/der-ascii tool first.

#[rustfmt::skip]
/// Generated from forty_two.der.
pub const FORTY_TWO: untrusted::Input = untrusted::Input::from(include_bytes!("der/forty_two.bin"));

#[rustfmt::skip]
/// Generated from false.der.
pub const FALSE: untrusted::Input = untrusted::Input::from(include_bytes!("der/false.bin"));

#[rustfmt::skip]
/// Generated from bits_partial_bad.der.
pub const BITS_PARTIAL_BAD: untrusted::Input = untrusted::Input::from(include_bytes!("der/bits_partial_bad.bin"));

#[rustfmt::skip]
/// Generated from bits_overflow.der.
pub const BITS_OVERFLOW: untrusted::Input = untrusted::Input::from(include_bytes!("der/bits_overflow.bin"));

#[rustfmt::skip]
/// Generated from huge_int.der.
pub const HUGE_INT: untrusted::Input = untrusted::Input::from(include_bytes!("der/huge_int.bin"));

#[rustfmt::skip]
/// Generated from indefinite_any.der.
pub const INDEFINITE_ANY: untrusted::Input = untrusted::Input::from(include_bytes!("der/indefinite_any.bin"));

#[rustfmt::skip]
/// Generated from long_form_tag.der.
pub const LONG_FORM_TAG: untrusted::Input = untrusted::Input::from(include_bytes!("der/long_form_tag.bin"));

#[rustfmt::skip]
/// Generated from negative.der.
pub const NEGATIVE: untrusted::Input = untrusted::Input::from(include_bytes!("der/negative.bin"));

#[rustfmt::skip]
/// Generated from one_twenty_eight.der.
pub const ONE_TWENTY_EIGHT: untrusted::Input = untrusted::Input::from(include_bytes!("der/one_twenty_eight.bin"));

#[rustfmt::skip]
/// Generated from bad_bool.der.
pub const BAD_BOOL: untrusted::Input = untrusted::Input::from(include_bytes!("der/bad_bool.bin"));

#[rustfmt::skip]
/// Generated from zero.der.
pub const ZERO: untrusted::Input = untrusted::Input::from(include_bytes!("der/zero.bin"));

#[rustfmt::skip]
/// Generated from nonempty_null.der.
pub const NONEMPTY_NULL: untrusted::Input = untrusted::Input::from(include_bytes!("der/nonempty_null.bin"));

#[rustfmt::skip]
/// Generated from bits_total.der.
pub const BITS_TOTAL: untrusted::Input = untrusted::Input::from(include_bytes!("der/bits_total.bin"));

#[rustfmt::skip]
/// Generated from null.der.
pub const NULL: untrusted::Input = untrusted::Input::from(include_bytes!("der/null.bin"));

#[rustfmt::skip]
/// Generated from true.der.
pub const TRUE: untrusted::Input = untrusted::Input::from(include_bytes!("der/true.bin"));

#[rustfmt::skip]
/// Generated from bits_partial.der.
pub const BITS_PARTIAL: untrusted::Input = untrusted::Input::from(include_bytes!("der/bits_partial.bin"));

#[rustfmt::skip]
/// Generated from short_form_any.der.
pub const SHORT_FORM_ANY: untrusted::Input = untrusted::Input::from(include_bytes!("der/short_form_any.bin"));

#[rustfmt::skip]
/// Generated from double_zero.der.
pub const DOUBLE_ZERO: untrusted::Input = untrusted::Input::from(include_bytes!("der/double_zero.bin"));

#[rustfmt::skip]
/// Generated from nine_thousand.der.
pub const NINE_THOUSAND: untrusted::Input = untrusted::Input::from(include_bytes!("der/nine_thousand.bin"));

#[rustfmt::skip]
/// Generated from long_form_any.der.
pub const LONG_FORM_ANY: untrusted::Input = untrusted::Input::from(include_bytes!("der/long_form_any.bin"));

#[rustfmt::skip]
/// Generated from empty.der.
pub const EMPTY: untrusted::Input = untrusted::Input::from(include_bytes!("der/empty.bin"));

#[rustfmt::skip]
/// Generated from x509_self_signed.der.
/// Signed with ./src/cert/testdata/../../crypto/testdata/rsa_2048_private_key.pk8.
pub const X509_SELF_SIGNED: untrusted::Input = untrusted::Input::from(include_bytes!("der/x509_self_signed.bin"));
