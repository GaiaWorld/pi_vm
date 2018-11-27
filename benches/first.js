var pi_modules = {};
// 定义基础函数模块
pi_modules.butil = { id: 'butil', exports: undefined, loaded: true };
pi_modules.butil.exports = (function () {
	var module = function mod_butil() { };
	// utf8的ArrayBuffer解码成字符串
	module.utf8Decode = (self.TextDecoder) ? (function () {
		var decoder = new TextDecoder('utf-8');
		return function (arr) {
			if((!arr) || arr.byteLength === 0)
				return "";
			if(arr instanceof ArrayBuffer)
				arr = new Uint8Array(arr);
			return decoder.decode(arr);
		};
	})() : function (arr) {
		if((!arr) || arr.byteLength === 0)
			return "";
		if(arr instanceof ArrayBuffer)
			arr = new Uint8Array(arr);
		var c, out = "", i = 0, len = arr.length;
		while (i < len) {
			c = arr[i++];
			if (c < 128) {
				out += String.fromCharCode(c);
			} else if (c < 0xE0 && i < len) {
				out += String.fromCharCode(((c & 0x1F) << 6) | (arr[i++] & 0x3F));
			} else if (c < 0xF0 && i + 1 < len) {
				out += String.fromCharCode((((c & 0x0F) << 12) | ((arr[i++] & 0x3F) << 6) | (arr[i++] & 0x3F)));
			} else if (c < 0xF8 && i + 2 < len) {
				out += String.fromCharCode((((c & 0x07) << 18) | ((arr[i++] & 0x3F) << 12) | ((arr[i++] & 0x3F) << 6) | (arr[i++] & 0x3F)));
			} else if (c < 0xFC && i + 3 < len) {
				out += String.fromCharCode((((c & 0x03) << 24) | ((arr[i++] & 0x3F) << 18) | ((arr[i++] & 0x3F) << 12) | ((arr[i++] & 0x3F) << 6) | (arr[i++] & 0x3F)));
			} else if (c < 0xFE && i + 4 < len) {
				out += String.fromCharCode((((c & 0x01) << 30) | ((arr[i++] & 0x3F) << 24) | ((arr[i++] & 0x3F) << 18) | ((arr[i++] & 0x3F) << 12) | ((arr[i++] & 0x3F) << 6) | (arr[i++] & 0x3F)));
			} else
				throw new Error("invalid utf8");
		}
		return out;
	};
	// 柯里化函数，将调用参数放在参数列表前
	module.curryFirst = function (func, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8) {
		return function (arg) {
			return func(arg, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8);
		};
	};
	// 柯里化函数，将调用参数放在参数列表后
	module.curryLast = function (func, arg/*:any*/) {
		return function (arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8) {
			return func(arg, arg1, arg2, arg3, arg4, arg5, arg6, arg7, arg8);
		};
	};
	// 获得分隔文件名字和后缀的点的位置
	module.fileDot = function (file/*:string*/) {
		var i, c;
		for (i = file.length - 1; i >= 0; i--) {
			c = file.charCodeAt(i);
			if (c === 47)
				return -1;
			if (c === 46)
				return i;
		}
		return -1;
	};
	// 获得文件后缀
	module.fileSuffix = function (file/*:string*/) {
		var dot = module.fileDot(file);
		return (dot >= 0) ? file.slice(dot + 1) : "";
	};
	// 获得指定的路径相对目录的路径
	module.relativePath = function (filePath/*:string*/, dir/*:string*/) {
		var i, len, j;
		if (filePath.charCodeAt(0) !== 46)
			return filePath;
		i = 0;
		len = filePath.length;
		j = dir.length - 1;
		if (dir.charCodeAt(j) !== 47) {
			j = dir.lastIndexOf("/");
		}
		while (i < len) {
			if (filePath.charCodeAt(i) !== 46)
				break;
			if (filePath.charCodeAt(i + 1) === 47) {// ./的情况
				i += 2;
				break;
			}
			if (filePath.charCodeAt(i + 1) !== 46 || filePath.charCodeAt(i + 2) !== 47)
				break;
			// ../的情况
			i += 3;
			j = dir.lastIndexOf("/", j - 1);
		}
		if (i > 0)
			filePath = filePath.slice(i);
		if (j < 0)
			return filePath;
		if (j < dir.length - 1)
			dir = dir.slice(0, j + 1);
		return dir + filePath;
	};

	return module;
})();

pi_modules.commonjs = { id: 'commonjs', exports: undefined, loaded: true };
pi_modules.commonjs.exports = (function () {
	var module = function mod_commonjs() { };
	var cmdClass = function commonjs_class() { };
	// ============================================================ 导入的模块、类、函数、常量
	var butil = pi_modules.butil.exports;

	// ------------------------------------------------------------ 导出的常量
	// ------------------------------------------------------------ 导出的多个类
	// ------------------------------------------------------------ 导出的静态函数
	/**
	 * @description js源码调试标志
	 * @example
	 */
	// 获得模块名
	module.modName = function (path) {
		var dot = butil.fileDot(path);
		return (dot >= 0) ? path.slice(0, dot) : path;
	};
	// 判断是否为内置模块
	module.isBase = function (modName) {
		return modName.indexOf("/") < 0;
	};
	/**
	 * @description 获取已经加载的模块，modName为相对路径， 如果模块名为"./**"，表示相对当前模块路径的模块，为"/**"表示绝对路径的模块，为"**"表示系统内置模块。返回模块的exports
	 * @example
	 */
	module.relativeGet = function (modName, dir) {
		if (module.isBase(modName))
            return pi_modules[modName];
        //console.log(butil.relativePath(modName, dir));
		return pi_modules[butil.relativePath(modName, dir)];
	};

	/**
	 * @description 定义和构建模块
	 * @example
	 */
	module.create = function (mods) {
		var mod, i = mods.length - 1;
		for (; i >= 0; i--) {
			mod = mods[i];
			pi_modules[mod.id] = mod;
			self.importScripts(mod.url);
		}
		build(mods);
	};

	// ------------------------------------------------------------ 本地函数
	// 构建模块
	var build = function (mods) {
		var i, mod, oldlen, len = mods.length;
		// 尽量按照依赖的次序构建模块
		do {
			oldlen = len;
			for (i = len - 1; i >= 0; i--) {
				mod = mods[i];
				if (!checkDepend(mod))
					continue;
				buildMod(mod);
				if (i < len - 1)
					mods[i] = mods[len - 1];
				len--;
			}
		} while (len < oldlen);
		mods.length = len;
		// 强行构建模块
		if (len > 0 && module.debug)
			console.log("cycle depend modules,", mods);
		for (i = len - 1; i >= 0; i--) {
			buildMod(mods[i]);
		}
	};
	// 构建模块
	var buildMod = function (mod) {
		var func = mod.buildFunc;
		if (func) {
			mod.buildFunc = undefined;
			// 构建模块
			func(butil.curryFirst(relativeBuild, mod.id), mod.exports, mod);
			mod.loaded = true;
		} else if (!mod.loaded)
			throw new Error("invalid amd mod: " + mod.id);
	};

	// 检查文件的依赖是否都已经就绪
	var checkDepend = function (srcMod) {
		var j, name, mod, arr = srcMod.children || [];
		for (j = arr.length - 1; j >= 0; j--) {
			mod = module.relativeGet(arr[j], srcMod.id);
			if ((!mod) || !mod.loaded)
				return false;
		}
		return true;
	};
	// 相对构建
	var relativeBuild = function (modName, dir) {
		var mod = module.relativeGet(modName, dir);
		if (!mod)
			throw new Error("invalid require: " + modName + ", from: " + dir);
		var func = mod.buildFunc;
		if (func) {
			mod.buildFunc = undefined;
			func(butil.curryFirst(relativeBuild, mod.id), mod.exports, mod);
			mod.loaded = true;
		}
		return mod.exports;
	};

	module.buildMod = buildMod;
	// ============================================================ 立即执行的代码

	return module;
})();
// 定义全局的模块定义函数
self._$define = function (name, func) {
	var mod = pi_modules[name];
	if(!mod)
		throw new Error("invalid define: " + name);
	mod.buildFunc = func;
};

// 浏览器环境中，设置初始的消息接收函数，要求模块数组的第一个必须为server模块
// v8环境中，需要使用该方法初始化模块，e必须类似: {data:{mods:[{id:"", url:"", exports:{}}, ...]}}
self.onmessage = function (e) {
	var data = e.data;
	try {
		var server = data.mods[0].exports;
		pi_modules.commonjs.exports.create(data.mods);
		server.name = data.name;
		self.onmessage = server.onmessage;
		return self.postMessage({ ok: true });
	}catch(ex){
		var r = self.postMessage({
			error: 10,
			stack: ex.stack,
			reason: "server worker, init error! "+ex.msg
		});
		self.close();
		return r;
	}
};

// 为了兼容v8环境，提供下列函数的兼容实现
self.close = self.close || function() {};
self.postMessage = self.postMessage || function(r) {
	return r;
};
self.importScripts = self.importScripts || function(url) {
	eval(self._$scriptMap[url]);
};

// v8环境中，需要使用该方法设置模块的代码，全都代码都设置成功后，才可以调用onmessage方法初始化模块
self._$scriptMap = {};
self.importScript = function(url, content) {
	self._$scriptMap[url] = content;
	return url;
}
