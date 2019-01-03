function test() {
    var i, index, r;

    for(i = 0; i < 1e5; i++) {
        index = callbacks.register(callback);
        r = NativeObject.call(0x1, [index, 0xffffffff, 0xffffffff]);
    }
    __gc();
}

function callback(x, y, z) {
    return true;
}