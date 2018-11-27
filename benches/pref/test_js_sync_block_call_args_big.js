var args = [0xffffffff, 0xffffffff, 0xffffffff, 0xffffffff, 0xffffffff, 0xffffffff, 0xffffffff, 0xffffffff, 0xffffffff, 0xffffffff]

function test() {
    var i;

    for(i = 0; i < 1e4; i++) {
        var r;
        r = NativeObject.call(0x1, args);
        r = __thread_yield();
    }
    __gc();
}