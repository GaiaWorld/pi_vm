function test() {
    var i, x, y;

    for(i = 0; i < 1e5; i++) {
        var r;
        r = NativeObject.call(0x1, [0xffffffff]);
        r = __thread_yield();
        x = _$tmp_var[0];
        y = _$tmp_var[1];
    }
    __gc()
}