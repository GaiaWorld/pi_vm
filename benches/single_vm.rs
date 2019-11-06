#![feature(test)]
#![deny(warnings)]

extern crate test;

extern crate pi_vm;

use std::sync::Arc;
use std::fs::File;
use std::io::prelude::*;

use test::Bencher;

use pi_vm::bonmgr::NativeObjsAuth;
use pi_vm::adapter::{register_native_object, JS};

//test-add-fastint
#[bench]
fn add_fastint(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-add-fastint.js");

    start(b, js);
}

//test-add-float
#[bench]
fn add_float(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-add-float.js");

    start(b, js);
}

//test-add-int
#[bench]
fn add_int(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-add-int.js");

    start(b, js);
}

//test-add-nan
#[bench]
fn add_nan(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-add-nan.js");

    start(b, js);
}

//test-add-nan-fastint
#[bench]
fn add_nan_fastint(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-add-nan-fastint.js");

    start(b, js);
}

//test-add-string
#[bench]
fn add_string(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-add-string.js");

    start(b, js);
}

//test-arith-add
#[bench]
fn arith_add(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-arith-add.js");

    start(b, js);
}

//test-arith-add-string
#[bench]
fn arith_add_string(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-arith-add-string.js");

    start(b, js);
}

//test-arith-div
#[bench]
fn arith_div(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-arith-div.js");

    start(b, js);
}

//test-arith-mod
#[bench]
fn arith_mod(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-arith-mod.js");

    start(b, js);
}

//test-arith-mul
#[bench]
fn arith_mul(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-arith-mul.js");

    start(b, js);
}

//test-arith-sub
#[bench]
fn arith_sub(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-arith-sub.js");

    start(b, js);
}

//test-array-append
#[bench]
fn array_append(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-array-append.js");

    start(b, js);
}

//test-array-cons-list
#[bench]
fn array_cons_list(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-array-cons-list.js");

    start(b, js);
}

//test-array-foreach
#[bench]
fn array_foreach(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-array-foreach.js");

    start(b, js);
}

//test-array-literal-3
#[bench]
fn array_literal_3(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-array-literal-3.js");

    start(b, js);
}

//test-array-literal-20
#[bench]
fn array_literal_20(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-array-literal-20.js");

    start(b, js);
}

//test-array-literal-100
#[bench]
fn array_literal_100(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-array-literal-100.js");

    start(b, js);
}

//test-array-pop
#[bench]
fn array_pop(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-array-pop.js");

    start(b, js);
}

//test-array-push
#[bench]
fn array_push(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-array-push.js");

    start(b, js);
}

//test-array-read
#[bench]
fn array_read(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-array-read.js");

    start(b, js);
}

//test-array-read-len-loop
#[bench]
fn array_read_len_loop(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-array-read-lenloop.js");

    start(b, js);
}

//test-array-sort
#[bench]
fn array_sort(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-array-sort.js");

    start(b, js);
}

//test-array-write
#[bench]
fn array_write(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-array-write.js");

    start(b, js);
}

//test-array-write-length
#[bench]
fn array_write_length(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-array-write-length.js");

    start(b, js);
}

//test-assign-add
#[bench]
fn assign_add(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-assign-add.js");

    start(b, js);
}

//test-assign-addto
#[bench]
fn assign_add_to(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-assign-addto.js");

    start(b, js);
}

//test-assign-addto-nan
#[bench]
fn assign_add_to_nan(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-assign-addto-nan.js");

    start(b, js);
}

//test-assign-boolean
#[bench]
fn assign_boolean(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-assign-boolean.js");

    start(b, js);
}

//test-assign-const
#[bench]
fn assign_const(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-assign-const.js");

    start(b, js);
}

//test-assign-const-int
#[bench]
fn assign_const_int(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-assign-const-int.js");

    start(b, js);
}

//test-assign-const-int2
#[bench]
fn assign_const_int2(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-assign-const-int2.js");

    start(b, js);
}

//test-assign-literal
#[bench]
fn assign_literal(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-assign-literal.js");

    start(b, js);
}

//test-assign-proplhs-reg
#[bench]
fn assign_proplhs_reg(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-assign-proplhs-reg.js");

    start(b, js);
}

//test-assign-proplhs
#[bench]
fn assign_proplhs(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-assign-proplhs.js");

    start(b, js);
}

//test-assign-reg
#[bench]
fn assign_reg(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-assign-reg.js");

    start(b, js);
}

//test-base64-decode
#[bench]
fn base64_decode(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-base64-decode.js");

    start(b, js);
}

//test-base64-decode-whitespace
#[bench]
fn base64_decode_whitespace(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-base64-decode-whitespace.js");

    start(b, js);
}

//test-base64-encode
#[bench]
fn base64_encode(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-base64-encode.js");

    start(b, js);
}

//test-bitwise-ops
#[bench]
fn bitwise_ops(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-bitwise-ops.js");

    start(b, js);
}

//test-break-fast
#[bench]
fn break_fast(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-break-fast.js");

    start(b, js);
}

//test-break-slow
#[bench]
fn break_slow(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-break-slow.js");

    start(b, js);
}

//test-buffer-float32array-write
#[bench]
fn buffer_float32array_write(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-buffer-float32array-write.js");

    start(b, js);
}

//test-buffer-nodejs-read
#[bench]
fn buffer_nodejs_read(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-buffer-nodejs-read.js");

    start(b, js);
}

//test-buffer-nodejs-write
#[bench]
fn buffer_nodejs_write(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-buffer-nodejs-write.js");

    start(b, js);
}

//test-buffer-object-read
#[bench]
fn buffer_object_read(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-buffer-object-read.js");

    start(b, js);
}

//test-buffer-object-write
#[bench]
fn buffer_object_write(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-buffer-object-write.js");

    start(b, js);
}

//test-buffer-plain-read
#[bench]
fn buffer_plain_read(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-buffer-plain-read.js");

    start(b, js);
}

//test-buffer-plain-write
#[bench]
fn buffer_plain_write(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-buffer-plain-write.js");

    start(b, js);
}

//test-call-apply
#[bench]
fn call_apply(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-call-apply.js");

    start(b, js);
}

//test-call-basic-1
#[bench]
fn call_basic1(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-call-basic-1.js");

    start(b, js);
}

//test-call-basic-2
#[bench]
fn call_basic2(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-call-basic-2.js");

    start(b, js);
}

//test-call-basic-3
#[bench]
fn call_basic3(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-call-basic-3.js");

    start(b, js);
}

//test-call-basic-4
#[bench]
fn call_basic4(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-call-basic-4.js");

    start(b, js);
}

//test-call-bound
#[bench]
fn call_bound(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-call-bound.js");

    start(b, js);
}

//test-call-bound-deep
#[bench]
fn call_bound_deep(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-call-bound-deep.js");

    start(b, js);
}

//test-call-call
#[bench]
fn call_call(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-call-call.js");

    start(b, js);
}

//test-call-native
#[bench]
fn call_native(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-call-native.js");

    start(b, js);
}

//test-call-prop
#[bench]
fn call_prop(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-call-prop.js");

    start(b, js);
}

//test-call-proxy-apply-1
#[bench]
fn call_proxy_apply1(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-call-proxy-apply-1.js");

    start(b, js);
}

//test-call-proxy-pass-1
#[bench]
fn call_proxy_pass1(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-call-proxy-pass-1.js");

    start(b, js);
}

//test-call-reg
#[bench]
fn call_reg(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-call-reg.js");

    start(b, js);
}

//test-call-reg-new
#[bench]
fn call_reg_new(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-call-reg-new.js");

    start(b, js);
}

//test-call-tail-1
#[bench]
fn call_tail1(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-call-tail-1.js");

    start(b, js);
}

//test-call-tail-2
#[bench]
fn call_tail2(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-call-tail-2.js");

    start(b, js);
}

//test-call-var
#[bench]
fn call_var(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-call-var.js");

    start(b, js);
}

//test-closure-inner-functions
#[bench]
fn closure_inner_functions(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-closure-inner-functions.js");

    start(b, js);
}

//test-compile-mandel
#[bench]
fn compile_mandel(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-compile-mandel.js");

    start(b, js);
}

//test-compile-mandel-nofrac
#[bench]
fn compile_mandel_nofrac(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-compile-mandel-nofrac.js");

    start(b, js);
}

//test-compile-mandel-short
#[bench]
fn compile_mandel_short(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-compile-mandel-short.js");

    start(b, js);
}

//test-compile-string-ascii
#[bench]
fn compile_string_ascii(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-compile-string-ascii.js");

    start(b, js);
}

//test-continue-fast
#[bench]
fn continue_fast(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-continue-fast.js");

    start(b, js);
}

//test-continue-slow
#[bench]
fn continue_slow(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-continue-slow.js");

    start(b, js);
}

//test-empty-loop
#[bench]
fn empty_loop(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-empty-loop.js");

    start(b, js);
}

//test-empty-loop-slowpath
#[bench]
fn empty_loop_slow_path(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-empty-loop-slowpath.js");

    start(b, js);
}

//test-empty-loop-step3
#[bench]
fn empty_loop_step3(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-empty-loop-step3.js");

    start(b, js);
}

//test-enum-basic
#[bench]
fn enum_basic(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-enum-basic.js");

    start(b, js);
}

//test-equals-fastint
#[bench]
fn equals_fastint(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-equals-fastint.js");

    start(b, js);
}

//test-equals-nonfastint
#[bench]
fn equals_nonfastint(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-equals-nonfastint.js");

    start(b, js);
}

//test-error-create
#[bench]
fn error_create(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-error-create.js");

    start(b, js);
}

//test-fib
#[bench]
fn fib(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-fib.js");

    start(b, js);
}

//test-fib-2
#[bench]
fn fib2(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-fib-2.js");

    start(b, js);
}

//test-func-bind
#[bench]
fn func_bind(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-func-bind.js");

    start(b, js);
}

//test-func-tostring
#[bench]
fn func_tostring(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-func-tostring.js");

    start(b, js);
}

//test-global-lookup
#[bench]
fn global_lookup(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-global-lookup.js");

    start(b, js);
}

//test-hex-decode
#[bench]
fn hex_decode(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-hex-decode.js");

    start(b, js);
}

//test-hex-encode
#[bench]
fn hex_encode(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-hex-encode.js");

    start(b, js);
}

//test-jc-serialize
#[bench]
fn jc_serialize(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-jc-serialize.js");

    start(b, js);
}

//test-jc-serialize-indented
#[bench]
fn jc_serialize_indented(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-jc-serialize-indented.js");

    start(b, js);
}

//test-json-parse-hex
#[bench]
fn json_parse_hex(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-json-parse-hex.js");

    start(b, js);
}

//test-json-parse-integer
#[bench]
fn json_parse_integer(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-json-parse-integer.js");

    start(b, js);
}

//test-json-parse-number
#[bench]
fn json_parse_number(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-json-parse-number.js");

    start(b, js);
}

//test-json-parse-string
#[bench]
fn json_parse_string(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-json-parse-string.js");

    start(b, js);
}

//test-json-serialize
#[bench]
fn json_serialize(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-json-serialize.js");

    start(b, js);
}

//test-json-serialize-fastpath-loop
#[bench]
fn json_serialize_fastpath_loop(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-json-serialize-fastpath-loop.js");

    start(b, js);
}

//test-json-serialize-forceslow
#[bench]
fn json_serialize_forceslow(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-json-serialize-forceslow.js");

    start(b, js);
}

//test-json-serialize-hex
#[bench]
fn json_serialize_hex(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-json-serialize-hex.js");

    start(b, js);
}

//test-json-serialize-indented
#[bench]
fn json_serialize_indented(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-json-serialize-indented.js");

    start(b, js);
}

//test-json-serialize-indented-deep25
#[bench]
fn json_serialize_indented_deep25(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-json-serialize-indented-deep25.js");

    start(b, js);
}

//test-json-serialize-indented-deep100
#[bench]
fn json_serialize_indented_deep100(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-json-serialize-indented-deep100.js");

    start(b, js);
}

//test-json-serialize-indented-deep500
#[bench]
fn json_serialize_indented_deep500(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-json-serialize-indented-deep500.js");

    start(b, js);
}

//test-json-serialize-jsonrpc-message
#[bench]
fn json_serialize_jsonrpc_message(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-json-serialize-jsonrpc-message.js");

    start(b, js);
}

//test-json-serialize-nofrac
#[bench]
fn json_serialize_nofrac(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-json-serialize-nofrac.js");

    start(b, js);
}

//test-json-serialize-plainbuf
#[bench]
fn json_serialize_plainbuf(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-json-serialize-plainbuf.js");

    start(b, js);
}

//test-json-serialize-slowpath-loop
#[bench]
fn json_serialize_slowpath_loop(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-json-serialize-slowpath-loop.js");

    start(b, js);
}

//test-json-string-bench
#[bench]
fn json_string_bench(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-json-string-bench.js");

    start(b, js);
}

//test-json-string-stringify
#[bench]
fn json_string_stringify(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-json-string-stringify.js");

    start(b, js);
}

//test-jx-serialize
#[bench]
fn jx_serialize(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-jx-serialize.js");

    start(b, js);
}

//test-jx-serialize-bufobj
#[bench]
fn jx_serialize_bufobj(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-jx-serialize-bufobj.js");

    start(b, js);
}

//test-jx-serialize-bufobj-forceslow
#[bench]
fn jx_serialize_bufobj_forceslow(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-jx-serialize-bufobj-forceslow.js");

    start(b, js);
}

//test-jx-serialize-indented
#[bench]
fn jx_serialize_indented(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-jx-serialize-indented.js");

    start(b, js);
}

//test-mandel
#[bench]
fn mandel(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-mandel.js");

    start(b, js);
}

//test-mandel-iter10-normal
#[bench]
fn mandel_iter10_normal(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-mandel-iter10-normal.js");

    start(b, js);
}

//test-mandel-iter10-promise
#[bench]
fn mandel_iter10_promise(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-mandel-iter10-promise.js");

    start(b, js);
}

//test-mandel-promise
#[bench]
fn mandel_promise(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-mandel-promise.js");

    start(b, js);
}

//test-math-clz32
#[bench]
fn math_clz32(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-math-clz32.js");

    start(b, js);
}

//test-misc-1dcell
#[bench]
fn misc_1dcell(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-misc-1dcell.js");

    start(b, js);
}

//test-object-garbage
#[bench]
fn object_garbage(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-object-garbage.js");

    start(b, js);
}

//test-object-garbage-2
#[bench]
fn object_garbage2(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-object-garbage-2.js");

    start(b, js);
}

//test-object-literal-3
#[bench]
fn object_literal3(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-object-literal-3.js");

    start(b, js);
}

//test-object-literal-20
#[bench]
fn object_literal20(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-object-literal-20.js");

    start(b, js);
}

//test-object-literal-100
#[bench]
fn object_literal100(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-object-literal-100.js");

    start(b, js);
}

//test-prop-read
#[bench]
fn prop_read(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-prop-read.js");

    start(b, js);
}

//test-prop-read-4
#[bench]
fn prop_read4(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-prop-read-4.js");

    start(b, js);
}

//test-prop-read-8
#[bench]
fn prop_read8(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-prop-read-8.js");

    start(b, js);
}

//test-prop-read-16
#[bench]
fn prop_read16(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-prop-read-16.js");

    start(b, js);
}

//test-prop-read-32
#[bench]
fn prop_read32(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-prop-read-32.js");

    start(b, js);
}

//test-prop-read-48
#[bench]
fn prop_read48(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-prop-read-48.js");

    start(b, js);
}

//test-prop-read-64
#[bench]
fn prop_read64(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-prop-read-64.js");

    start(b, js);
}

//test-prop-read-256
#[bench]
fn prop_read256(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-prop-read-256.js");

    start(b, js);
}

//test-prop-read-1024
#[bench]
fn prop_read1024(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-prop-read-1024.js");

    start(b, js);
}

//test-prop-read-inherited
#[bench]
fn prop_read_inherited(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-prop-read-inherited.js");

    start(b, js);
}

//test-prop-write
#[bench]
fn prop_write(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-prop-read.js");

    start(b, js);
}

//test-prop-write-4
#[bench]
fn prop_write4(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-prop-write-4.js");

    start(b, js);
}

//test-prop-write-8
#[bench]
fn prop_write8(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-prop-write-8.js");

    start(b, js);
}

//test-prop-write-16
#[bench]
fn prop_write16(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-prop-write-16.js");

    start(b, js);
}

//test-prop-write-32
#[bench]
fn prop_write32(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-prop-write-32.js");

    start(b, js);
}

//test-prop-write-48
#[bench]
fn prop_write48(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-prop-write-48.js");

    start(b, js);
}

//test-prop-write-64
#[bench]
fn prop_write64(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-prop-write-64.js");

    start(b, js);
}

//test-prop-write-256
#[bench]
fn prop_write256(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-prop-write-256.js");

    start(b, js);
}

//test-prop-write-1024
#[bench]
fn prop_write1024(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-prop-write-1024.js");

    start(b, js);
}

//test-proxy-get
#[bench]
fn proxy_get(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-proxy-get.js");

    start(b, js);
}

//test-random
#[bench]
fn random(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-random.js");

    start(b, js);
}

//test-reflect-ownkeys-sorted
#[bench]
fn reflect_ownkeys_sorted(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-reflect-ownkeys-sorted.js");

    start(b, js);
}

//test-reflect-ownkeys-unsorted
#[bench]
fn reflect_ownkeys_unsorted(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-reflect-ownkeys-unsorted.js");

    start(b, js);
}

//test-regexp-case-insensitive-compile
#[bench]
fn regexp_case_insensitive_compile(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-regexp-case-insensitive-compile.js");

    start(b, js);
}

//test-regexp-case-insensitive-execute
#[bench]
fn regexp_case_insensitive_execute(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-regexp-case-insensitive-execute.js");

    start(b, js);
}

//test-regexp-case-sensitive-compile
#[bench]
fn regexp_case_sensitive_compile(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-regexp-case-sensitive-compile.js");

    start(b, js);
}

//test-regexp-case-sensitive-execute
#[bench]
fn regexp_case_sensitive_execute(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-regexp-case-insensitive-compile.js");

    start(b, js);
}

//test-regexp-compile
#[bench]
fn regexp_compile(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-regexp-compile.js");

    start(b, js);
}

//test-regexp-execute
#[bench]
fn regexp_execute(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-regexp-execute.js");

    start(b, js);
}

//test-regexp-string-parse
#[bench]
fn regexp_string_parse(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-regexp-string-parse.js");

    start(b, js);
}

//test-reg-readwrite-object
#[bench]
fn reg_readwrite_object(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-reg-readwrite-object.js");

    start(b, js);
}

//test-reg-readwrite-plain
#[bench]
fn reg_readwrite_plain(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-reg-readwrite-plain.js");

    start(b, js);
}

//test-strict-equals-fastint
#[bench]
fn strict_equals_fastint(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-strict-equals-fastint.js");

    start(b, js);
}

//test-strict-equals-nonfastint
#[bench]
fn strict_equals_nonfastint(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-strict-equals-nonfastint.js");

    start(b, js);
}

//test-string-array-concat
#[bench]
fn string_array_concat(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-string-array-concat.js");

    start(b, js);
}

//test-string-arridx
#[bench]
fn string_arridx(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-string-arridx.js");

    start(b, js);
}

//test-string-charlen-ascii
#[bench]
fn string_charlen_ascii(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-string-charlen-ascii.js");

    start(b, js);
}

//test-string-charlen-nonascii
#[bench]
fn string_charlen_nonascii(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-string-charlen-nonascii.js");

    start(b, js);
}

//test-string-compare
#[bench]
fn string_compare(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-string-compare.js");

    start(b, js);
}

//test-string-decodeuri
#[bench]
fn string_decodeuri(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-string-decodeuri.js");

    start(b, js);
}

//test-string-encodeuri
#[bench]
fn string_encodeuri(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-string-encodeuri.js");

    start(b, js);
}

//test-string-garbage
#[bench]
fn string_garbage(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-string-garbage.js");

    start(b, js);
}

//test-string-intern-grow
#[bench]
fn string_intern_grow(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-string-intern-grow.js");

    start(b, js);
}

//test-string-intern-grow2
#[bench]
fn string_intern_grow2(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-string-intern-grow2.js");

    start(b, js);
}

//test-string-intern-grow-short
#[bench]
fn string_intern_grow_short(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-string-intern-grow-short.js");

    start(b, js);
}

//test-string-intern-grow-short2
#[bench]
fn string_intern_grow_short2(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-string-intern-grow-short2.js");

    start(b, js);
}

//test-string-intern-match
#[bench]
fn string_intern_match(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-string-intern-match.js");

    start(b, js);
}

//test-string-intern-match-short
#[bench]
fn string_intern_match_short(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-string-intern-match-short.js");

    start(b, js);
}

//test-string-intern-miss
#[bench]
fn string_intern_miss(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-string-intern-miss.js");

    start(b, js);
}

//test-string-intern-miss-short
#[bench]
fn string_intern_miss_short(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-string-intern-miss-short.js");

    start(b, js);
}

//test-string-literal-intern
#[bench]
fn string_literal_intern(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-string-literal-intern.js");

    start(b, js);
}

//test-string-number-list
#[bench]
fn string_number_list(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-string-number-list.js");

    start(b, js);
}

//test-string-plain-concat
#[bench]
fn string_plain_concat(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-string-plain-concat.js");

    start(b, js);
}

//test-string-scan-nonascii
#[bench]
fn string_scan_nonascii(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-string-scan-nonascii.js");

    start(b, js);
}

//test-string-uppercase
#[bench]
fn string_uppercase(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-string-uppercase.js");

    start(b, js);
}

//test-symbol-tostring
#[bench]
fn symbol_tostring(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-symbol-tostring.js");

    start(b, js);
}

//test-textdecoder-ascii
#[bench]
fn textdecoder_ascii(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-textdecoder-ascii.js");

    start(b, js);
}

//test-textdecoder-nonascii
#[bench]
fn textdecoder_nonascii(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-textdecoder-nonascii.js");

    start(b, js);
}

//test-textencoder-ascii
#[bench]
fn textencoder_ascii(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-textencoder-ascii.js");

    start(b, js);
}

//test-textencoder-nonascii
#[bench]
fn textencoder_nonascii(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-textencoder-nonascii.js");

    start(b, js);
}

//test-try-catch-nothrow
#[bench]
fn try_catch_nothrow(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-try-catch-nothrow.js");

    start(b, js);
}

//test-try-catch-throw
#[bench]
fn try_catch_throw(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-try-catch-throw.js");

    start(b, js);
}

//test-try-finally-nothrow
#[bench]
fn try_finally_nothrow(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-try-finally-nothrow.js");

    start(b, js);
}

//test-try-finally-throw
#[bench]
fn try_finally_throw(b: &mut Bencher) {
    register_native_object();

    let js = create_js();
    load_js(js.clone(), "benches/pref/test-try-finally-throw.js");

    start(b, js);
}

//创建虚拟机
fn create_js() -> Arc<JS> {
    if let Some(js) = JS::new(Arc::new(NativeObjsAuth::new(None, None))) {
        load_js(js.clone(), "benches/core.js");
        return js;
    }
    panic!("!!!> Create Vm Error");
}

//读取指定js文件，并在指定虚拟机上编译、加载并运行
fn load_js(js: Arc<JS>, file: &str) {
    let file_name = &String::from(file);
    if let Ok(mut file) = File::open(file) {
        let mut contents = String::new();
        if let Ok(_) = file.read_to_string(&mut contents) {
            if let Some(ref code) = js.compile(file_name.clone(), (&contents).clone()) {
                return assert!(js.load(code));
            }
        }
    }
    panic!("!!!> Load Script Error");
}

//开始测试
fn start(b: &mut Bencher, js: Arc<JS>) {
    b.iter(|| {
        js.get_js_function("test".to_string());
        js.call(0);
    });
}