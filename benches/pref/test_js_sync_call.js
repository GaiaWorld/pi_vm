function test() {
    var i;

    for(i = 0; i < 1e4; i++) {
        var r;
        try {
            r = NativeObject.call(0x1, [0xffffffff]);
        } catch(e) {

        }
    }
    __gc();
}