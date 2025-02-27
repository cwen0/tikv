// Copyright 2019 TiKV Project Authors. Licensed under Apache-2.0.

pub mod types;

pub mod impl_arithmetic;
pub mod impl_cast;
pub mod impl_compare;
pub mod impl_control;
pub mod impl_json;
pub mod impl_like;
pub mod impl_math;
pub mod impl_op;
pub mod impl_time;

pub use self::types::*;

use tidb_query_datatype::{FieldTypeAccessor, FieldTypeFlag};
use tipb::{Expr, ScalarFuncSig};

use crate::codec::data_type::*;
use crate::Result;

use self::impl_arithmetic::*;
use self::impl_compare::*;
use self::impl_control::*;
use self::impl_json::*;
use self::impl_like::*;
use self::impl_math::*;
use self::impl_op::*;
use self::impl_time::*;

fn map_int_sig<F>(value: ScalarFuncSig, children: &[Expr], mapper: F) -> Result<RpnFnMeta>
where
    F: Fn(bool, bool) -> RpnFnMeta,
{
    // FIXME: The signature for different signed / unsigned int should be inferred at TiDB side.
    if children.len() != 2 {
        return Err(other_err!(
            "ScalarFunction {:?} (params = {}) is not supported in batch mode",
            value,
            children.len()
        ));
    }
    let lhs_is_unsigned = children[0]
        .get_field_type()
        .as_accessor()
        .flag()
        .contains(FieldTypeFlag::UNSIGNED);
    let rhs_is_unsigned = children[1]
        .get_field_type()
        .as_accessor()
        .flag()
        .contains(FieldTypeFlag::UNSIGNED);
    Ok(mapper(lhs_is_unsigned, rhs_is_unsigned))
}

fn compare_mapper<F: CmpOp>(lhs_is_unsigned: bool, rhs_is_unsigned: bool) -> RpnFnMeta {
    match (lhs_is_unsigned, rhs_is_unsigned) {
        (false, false) => compare_fn_meta::<BasicComparer<Int, F>>(),
        (false, true) => compare_fn_meta::<IntUintComparer<F>>(),
        (true, false) => compare_fn_meta::<UintIntComparer<F>>(),
        (true, true) => compare_fn_meta::<UintUintComparer<F>>(),
    }
}

fn plus_mapper(lhs_is_unsigned: bool, rhs_is_unsigned: bool) -> RpnFnMeta {
    match (lhs_is_unsigned, rhs_is_unsigned) {
        (false, false) => arithmetic_fn_meta::<IntIntPlus>(),
        (false, true) => arithmetic_fn_meta::<IntUintPlus>(),
        (true, false) => arithmetic_fn_meta::<UintIntPlus>(),
        (true, true) => arithmetic_fn_meta::<UintUintPlus>(),
    }
}

fn minus_mapper(lhs_is_unsigned: bool, rhs_is_unsigned: bool) -> RpnFnMeta {
    match (lhs_is_unsigned, rhs_is_unsigned) {
        (false, false) => arithmetic_fn_meta::<IntIntMinus>(),
        (false, true) => arithmetic_fn_meta::<IntUintMinus>(),
        (true, false) => arithmetic_fn_meta::<UintIntMinus>(),
        (true, true) => arithmetic_fn_meta::<UintUintMinus>(),
    }
}

fn multiply_mapper(lhs_is_unsigned: bool, rhs_is_unsigned: bool) -> RpnFnMeta {
    match (lhs_is_unsigned, rhs_is_unsigned) {
        (false, false) => arithmetic_fn_meta::<IntIntMultiply>(),
        (false, true) => arithmetic_fn_meta::<IntUintMultiply>(),
        (true, false) => arithmetic_fn_meta::<UintIntMultiply>(),
        (true, true) => arithmetic_fn_meta::<UintUintMultiply>(),
    }
}

fn mod_mapper(lhs_is_unsigned: bool, rhs_is_unsigned: bool) -> RpnFnMeta {
    match (lhs_is_unsigned, rhs_is_unsigned) {
        (false, false) => arithmetic_fn_meta::<IntIntMod>(),
        (false, true) => arithmetic_fn_meta::<IntUintMod>(),
        (true, false) => arithmetic_fn_meta::<UintIntMod>(),
        (true, true) => arithmetic_fn_meta::<UintUintMod>(),
    }
}

fn divide_mapper(lhs_is_unsigned: bool, rhs_is_unsigned: bool) -> RpnFnMeta {
    match (lhs_is_unsigned, rhs_is_unsigned) {
        (false, false) => arithmetic_fn_meta::<IntDivideInt>(),
        (false, true) => arithmetic_fn_meta::<IntDivideUint>(),
        (true, false) => arithmetic_fn_meta::<UintDivideInt>(),
        (true, true) => arithmetic_fn_meta::<UintDivideUint>(),
    }
}

#[rustfmt::skip]
fn map_pb_sig_to_rpn_func(value: ScalarFuncSig, children: &[Expr]) -> Result<RpnFnMeta> {
    Ok(match value {
        ScalarFuncSig::LTInt => map_int_sig(value, children, compare_mapper::<CmpOpLT>)?,
        ScalarFuncSig::LTReal => compare_fn_meta::<BasicComparer<Real, CmpOpLT>>(),
        ScalarFuncSig::LTDecimal => compare_fn_meta::<BasicComparer<Decimal, CmpOpLT>>(),
        ScalarFuncSig::LTString => compare_fn_meta::<BasicComparer<Bytes, CmpOpLT>>(),
        ScalarFuncSig::LTTime => compare_fn_meta::<BasicComparer<DateTime, CmpOpLT>>(),
        ScalarFuncSig::LTDuration => compare_fn_meta::<BasicComparer<Duration, CmpOpLT>>(),
        ScalarFuncSig::LTJson => compare_fn_meta::<BasicComparer<Json, CmpOpLT>>(),
        ScalarFuncSig::LEInt => map_int_sig(value, children, compare_mapper::<CmpOpLE>)?,
        ScalarFuncSig::LEReal => compare_fn_meta::<BasicComparer<Real, CmpOpLE>>(),
        ScalarFuncSig::LEDecimal => compare_fn_meta::<BasicComparer<Decimal, CmpOpLE>>(),
        ScalarFuncSig::LEString => compare_fn_meta::<BasicComparer<Bytes, CmpOpLE>>(),
        ScalarFuncSig::LETime => compare_fn_meta::<BasicComparer<DateTime, CmpOpLE>>(),
        ScalarFuncSig::LEDuration => compare_fn_meta::<BasicComparer<Duration, CmpOpLE>>(),
        ScalarFuncSig::LEJson => compare_fn_meta::<BasicComparer<Json, CmpOpLE>>(),
        ScalarFuncSig::GTInt => map_int_sig(value, children, compare_mapper::<CmpOpGT>)?,
        ScalarFuncSig::GTReal => compare_fn_meta::<BasicComparer<Real, CmpOpGT>>(),
        ScalarFuncSig::GTDecimal => compare_fn_meta::<BasicComparer<Decimal, CmpOpGT>>(),
        ScalarFuncSig::GTString => compare_fn_meta::<BasicComparer<Bytes, CmpOpGT>>(),
        ScalarFuncSig::GTTime => compare_fn_meta::<BasicComparer<DateTime, CmpOpGT>>(),
        ScalarFuncSig::GTDuration => compare_fn_meta::<BasicComparer<Duration, CmpOpGT>>(),
        ScalarFuncSig::GTJson => compare_fn_meta::<BasicComparer<Json, CmpOpGT>>(),
        ScalarFuncSig::GEInt => map_int_sig(value, children, compare_mapper::<CmpOpGE>)?,
        ScalarFuncSig::GEReal => compare_fn_meta::<BasicComparer<Real, CmpOpGE>>(),
        ScalarFuncSig::GEDecimal => compare_fn_meta::<BasicComparer<Decimal, CmpOpGE>>(),
        ScalarFuncSig::GEString => compare_fn_meta::<BasicComparer<Bytes, CmpOpGE>>(),
        ScalarFuncSig::GETime => compare_fn_meta::<BasicComparer<DateTime, CmpOpGE>>(),
        ScalarFuncSig::GEDuration => compare_fn_meta::<BasicComparer<Duration, CmpOpGE>>(),
        ScalarFuncSig::GEJson => compare_fn_meta::<BasicComparer<Json, CmpOpGE>>(),
        ScalarFuncSig::NEInt => map_int_sig(value, children, compare_mapper::<CmpOpNE>)?,
        ScalarFuncSig::NEReal => compare_fn_meta::<BasicComparer<Real, CmpOpNE>>(),
        ScalarFuncSig::NEDecimal => compare_fn_meta::<BasicComparer<Decimal, CmpOpNE>>(),
        ScalarFuncSig::NEString => compare_fn_meta::<BasicComparer<Bytes, CmpOpNE>>(),
        ScalarFuncSig::NETime => compare_fn_meta::<BasicComparer<DateTime, CmpOpNE>>(),
        ScalarFuncSig::NEDuration => compare_fn_meta::<BasicComparer<Duration, CmpOpNE>>(),
        ScalarFuncSig::NEJson => compare_fn_meta::<BasicComparer<Json, CmpOpNE>>(),
        ScalarFuncSig::EQInt => map_int_sig(value, children, compare_mapper::<CmpOpEQ>)?,
        ScalarFuncSig::EQReal => compare_fn_meta::<BasicComparer<Real, CmpOpEQ>>(),
        ScalarFuncSig::EQDecimal => compare_fn_meta::<BasicComparer<Decimal, CmpOpEQ>>(),
        ScalarFuncSig::EQString => compare_fn_meta::<BasicComparer<Bytes, CmpOpEQ>>(),
        ScalarFuncSig::EQTime => compare_fn_meta::<BasicComparer<DateTime, CmpOpEQ>>(),
        ScalarFuncSig::EQDuration => compare_fn_meta::<BasicComparer<Duration, CmpOpEQ>>(),
        ScalarFuncSig::EQJson => compare_fn_meta::<BasicComparer<Json, CmpOpEQ>>(),
        ScalarFuncSig::NullEQInt => map_int_sig(value, children, compare_mapper::<CmpOpNullEQ>)?,
        ScalarFuncSig::NullEQReal => compare_fn_meta::<BasicComparer<Real, CmpOpNullEQ>>(),
        ScalarFuncSig::NullEQDecimal => compare_fn_meta::<BasicComparer<Decimal, CmpOpNullEQ>>(),
        ScalarFuncSig::NullEQString => compare_fn_meta::<BasicComparer<Bytes, CmpOpNullEQ>>(),
        ScalarFuncSig::NullEQTime => compare_fn_meta::<BasicComparer<DateTime, CmpOpNullEQ>>(),
        ScalarFuncSig::NullEQDuration => compare_fn_meta::<BasicComparer<Duration, CmpOpNullEQ>>(),
        ScalarFuncSig::NullEQJson => compare_fn_meta::<BasicComparer<Json, CmpOpNullEQ>>(),
        ScalarFuncSig::IntIsNull => is_null_fn_meta::<Int>(),
        ScalarFuncSig::RealIsNull => is_null_fn_meta::<Real>(),
        ScalarFuncSig::DecimalIsNull => is_null_fn_meta::<Decimal>(),
        ScalarFuncSig::StringIsNull => is_null_fn_meta::<Bytes>(),
        ScalarFuncSig::TimeIsNull => is_null_fn_meta::<DateTime>(),
        ScalarFuncSig::DurationIsNull => is_null_fn_meta::<Duration>(),
        ScalarFuncSig::JsonIsNull => is_null_fn_meta::<Json>(),
        ScalarFuncSig::IntIsTrue => int_is_true_fn_meta(),
        ScalarFuncSig::RealIsTrue => real_is_true_fn_meta(),
        ScalarFuncSig::DecimalIsTrue => decimal_is_true_fn_meta(),
        ScalarFuncSig::IntIsFalse => int_is_false_fn_meta(),
        ScalarFuncSig::RealIsFalse => real_is_false_fn_meta(),
        ScalarFuncSig::DecimalIsFalse => decimal_is_false_fn_meta(),
        ScalarFuncSig::LogicalAnd => logical_and_fn_meta(),
        ScalarFuncSig::LogicalOr => logical_or_fn_meta(),
        ScalarFuncSig::UnaryNotInt | ScalarFuncSig::UnaryNotDecimal | ScalarFuncSig::UnaryNotReal => unary_not_fn_meta(),
        ScalarFuncSig::PlusInt => map_int_sig(value, children, plus_mapper)?,
        ScalarFuncSig::PlusReal => arithmetic_fn_meta::<RealPlus>(),
        ScalarFuncSig::PlusDecimal => arithmetic_fn_meta::<DecimalPlus>(),
        ScalarFuncSig::MinusInt => map_int_sig(value, children, minus_mapper)?,
        ScalarFuncSig::MinusReal => arithmetic_fn_meta::<RealMinus>(),
        ScalarFuncSig::MinusDecimal => arithmetic_fn_meta::<DecimalMinus>(),
        ScalarFuncSig::MultiplyDecimal => arithmetic_fn_meta::<DecimalMultiply>(),
        ScalarFuncSig::MultiplyInt => map_int_sig(value, children, multiply_mapper)?,
        ScalarFuncSig::MultiplyIntUnsigned => arithmetic_fn_meta::<UintUintMultiply>(),
        ScalarFuncSig::MultiplyReal => arithmetic_fn_meta::<RealMultiply>(),
        ScalarFuncSig::ModReal => arithmetic_fn_meta::<RealMod>(),
        ScalarFuncSig::ModDecimal => arithmetic_fn_meta::<DecimalMod>(),
        ScalarFuncSig::DivideDecimal => arithmetic_with_ctx_fn_meta::<DecimalDivide>(),
        ScalarFuncSig::DivideReal => arithmetic_with_ctx_fn_meta::<RealDivide>(),
        ScalarFuncSig::ModInt => map_int_sig(value, children, mod_mapper)?,
        ScalarFuncSig::LikeSig => like_fn_meta(),
        ScalarFuncSig::IfNullInt => if_null_fn_meta::<Int>(),
        ScalarFuncSig::IfNullReal => if_null_fn_meta::<Real>(),
        ScalarFuncSig::IfNullString => if_null_fn_meta::<Bytes>(),
        ScalarFuncSig::IfNullDecimal => if_null_fn_meta::<Decimal>(),
        ScalarFuncSig::IfNullTime => if_null_fn_meta::<DateTime>(),
        ScalarFuncSig::IfNullDuration => if_null_fn_meta::<Duration>(),
        ScalarFuncSig::IfNullJson => if_null_fn_meta::<Json>(),
        ScalarFuncSig::IntDivideInt => map_int_sig(value, children, divide_mapper)?,
        ScalarFuncSig::IntDivideDecimal => int_divide_decimal_fn_meta(),
        ScalarFuncSig::CaseWhenInt => case_when_fn_meta::<Int>(),
        ScalarFuncSig::CaseWhenReal => case_when_fn_meta::<Real>(),
        ScalarFuncSig::CaseWhenString => case_when_fn_meta::<Bytes>(),
        ScalarFuncSig::CaseWhenDecimal => case_when_fn_meta::<Decimal>(),
        ScalarFuncSig::CaseWhenTime => case_when_fn_meta::<DateTime>(),
        ScalarFuncSig::CaseWhenDuration => case_when_fn_meta::<Duration>(),
        ScalarFuncSig::CaseWhenJson => case_when_fn_meta::<Json>(),
        ScalarFuncSig::DateFormatSig => date_format_fn_meta(),
        ScalarFuncSig::AbsInt => abs_int_fn_meta(),
        ScalarFuncSig::AbsUInt => abs_uint_fn_meta(),
        ScalarFuncSig::AbsReal => abs_real_fn_meta(),
        ScalarFuncSig::AbsDecimal => abs_decimal_fn_meta(),
        ScalarFuncSig::CoalesceInt => coalesce_fn_meta::<Int>(),
        ScalarFuncSig::CoalesceReal => coalesce_fn_meta::<Real>(),
        ScalarFuncSig::CoalesceString => coalesce_fn_meta::<Bytes>(),
        ScalarFuncSig::CoalesceDecimal => coalesce_fn_meta::<Decimal>(),
        ScalarFuncSig::CoalesceTime => coalesce_fn_meta::<DateTime>(),
        ScalarFuncSig::CoalesceDuration => coalesce_fn_meta::<Duration>(),
        ScalarFuncSig::CoalesceJson => coalesce_fn_meta::<Json>(),
        ScalarFuncSig::InInt => compare_in_fn_meta::<Int>(),
        ScalarFuncSig::InReal => compare_in_fn_meta::<Real>(),
        ScalarFuncSig::InString => compare_in_fn_meta::<Bytes>(),
        ScalarFuncSig::InDecimal => compare_in_fn_meta::<Decimal>(),
        ScalarFuncSig::InTime => compare_in_fn_meta::<DateTime>(),
        ScalarFuncSig::InDuration => compare_in_fn_meta::<Duration>(),
        ScalarFuncSig::InJson => compare_in_fn_meta::<Json>(),
        ScalarFuncSig::IfReal => if_condition_fn_meta::<Real>(),
        ScalarFuncSig::IfJson => if_condition_fn_meta::<Json>(),
        ScalarFuncSig::IfInt => if_condition_fn_meta::<Int>(),
        ScalarFuncSig::IfDuration => if_condition_fn_meta::<Duration>(),
        ScalarFuncSig::IfString => if_condition_fn_meta::<Bytes>(),
        ScalarFuncSig::IfTime => if_condition_fn_meta::<DateTime>(),
        ScalarFuncSig::IfDecimal => if_condition_fn_meta::<Decimal>(),
        ScalarFuncSig::JsonTypeSig => json_type_fn_meta(),
        _ => return Err(other_err!(
            "ScalarFunction {:?} is not supported in batch mode",
            value
        )),
    })
}
