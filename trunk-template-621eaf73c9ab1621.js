let Q=0,S=`string`,R=1,U=`Object`,O=`utf-8`,M=null,N=`undefined`,W=4,T=`function`,K=128,J=Array,P=Error,V=FinalizationRegistry,X=Object,L=undefined;var D=(async(a,b)=>{if(typeof Response===T&&a instanceof Response){if(typeof WebAssembly.instantiateStreaming===T){try{return await WebAssembly.instantiateStreaming(a,b)}catch(b){if(a.headers.get(`Content-Type`)!=`application/wasm`){console.warn(`\`WebAssembly.instantiateStreaming\` failed because your server does not serve wasm with \`application/wasm\` MIME type. Falling back to \`WebAssembly.instantiate\` which is slower. Original error:\\n`,b)}else{throw b}}};const c=await a.arrayBuffer();return await WebAssembly.instantiate(c,b)}else{const c=await WebAssembly.instantiate(a,b);if(c instanceof WebAssembly.Instance){return {instance:c,module:a}}else{return c}}});var l=(a=>{const b=typeof a;if(b==`number`||b==`boolean`||a==M){return `${a}`};if(b==S){return `"${a}"`};if(b==`symbol`){const b=a.description;if(b==M){return `Symbol`}else{return `Symbol(${b})`}};if(b==T){const b=a.name;if(typeof b==S&&b.length>Q){return `Function(${b})`}else{return `Function`}};if(J.isArray(a)){const b=a.length;let c=`[`;if(b>Q){c+=l(a[Q])};for(let d=R;d<b;d++){c+=`, `+ l(a[d])};c+=`]`;return c};const c=/\[object ([^\]]+)\]/.exec(toString.call(a));let d;if(c.length>R){d=c[R]}else{return toString.call(a)};if(d==U){try{return `Object(`+ JSON.stringify(a)+ `)`}catch(a){return U}};if(a instanceof P){return `${a.name}: ${a.message}\n${a.stack}`};return d});var F=((a,b)=>{});var B=((a,b)=>{a=a>>>Q;const c=A();const d=c.subarray(a/W,a/W+ b);const e=[];for(let a=Q;a<d.length;a++){e.push(f(d[a]))};return e});var k=(a=>{if(d===b.length)b.push(b.length+ R);const c=d;d=b[c];b[c]=a;return c});var f=(a=>{const b=c(a);e(a);return b});var w=((c,d,e)=>{try{a._dyn_core__ops__function__FnMut___A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h8c8dbd81f851b2de(c,d,v(e))}finally{b[u++]=L}});function C(b,c){try{return b.apply(this,c)}catch(b){a.__wbindgen_exn_store(k(b))}}var I=(async(b)=>{if(a!==L)return a;if(typeof b===N){b=new URL(`trunk-template-621eaf73c9ab1621_bg.wasm`,import.meta.url)};const c=E();if(typeof b===S||typeof Request===T&&b instanceof Request||typeof URL===T&&b instanceof URL){b=fetch(b)};F(c);const {instance:d,module:e}=await D(await b,c);return G(d,e)});var r=(()=>{if(q===M||q.byteLength===Q){q=new Int32Array(a.memory.buffer)};return q});var y=(a=>a===L||a===M);var c=(a=>b[a]);var H=(b=>{if(a!==L)return a;const c=E();F(c);if(!(b instanceof WebAssembly.Module)){b=new WebAssembly.Module(b)};const d=new WebAssembly.Instance(b,c);return G(d,b)});var G=((b,c)=>{a=b.exports;I.__wbindgen_wasm_module=c;q=M;z=M;h=M;a.__wbindgen_start();return a});var E=(()=>{const b={};b.wbg={};b.wbg.__wbindgen_object_drop_ref=(a=>{f(a)});b.wbg.__wbindgen_string_new=((a,b)=>{const c=j(a,b);return k(c)});b.wbg.__wbindgen_object_clone_ref=(a=>{const b=c(a);return k(b)});b.wbg.__wbg_listenerid_12315eee21527820=((a,b)=>{const d=c(b).__yew_listener_id;r()[a/W+ R]=y(d)?Q:d;r()[a/W+ Q]=!y(d)});b.wbg.__wbg_setlistenerid_3183aae8fa5840fb=((a,b)=>{c(a).__yew_listener_id=b>>>Q});b.wbg.__wbg_subtreeid_e348577f7ef777e3=((a,b)=>{const d=c(b).__yew_subtree_id;r()[a/W+ R]=y(d)?Q:d;r()[a/W+ Q]=!y(d)});b.wbg.__wbg_setsubtreeid_d32e6327eef1f7fc=((a,b)=>{c(a).__yew_subtree_id=b>>>Q});b.wbg.__wbg_cachekey_b61393159c57fd7b=((a,b)=>{const d=c(b).__yew_subtree_cache_key;r()[a/W+ R]=y(d)?Q:d;r()[a/W+ Q]=!y(d)});b.wbg.__wbg_setcachekey_80183b7cfc421143=((a,b)=>{c(a).__yew_subtree_cache_key=b>>>Q});b.wbg.__wbg_new_abda76e883ba8a5f=(()=>{const a=new P();return k(a)});b.wbg.__wbg_stack_658279fe44541cf6=((b,d)=>{const e=c(d).stack;const f=p(e,a.__wbindgen_malloc,a.__wbindgen_realloc);const g=m;r()[b/W+ R]=g;r()[b/W+ Q]=f});b.wbg.__wbg_error_f851667af71bcfc6=((b,c)=>{let d;let e;try{d=b;e=c;console.error(j(b,c))}finally{a.__wbindgen_free(d,e,R)}});b.wbg.__wbindgen_cb_drop=(a=>{const b=f(a).original;if(b.cnt--==R){b.a=Q;return !0};const c=!1;return c});b.wbg.__wbg_queueMicrotask_f61ee94ee663068b=(a=>{queueMicrotask(c(a))});b.wbg.__wbg_queueMicrotask_f82fc5d1e8f816ae=(a=>{const b=c(a).queueMicrotask;return k(b)});b.wbg.__wbindgen_is_function=(a=>{const b=typeof c(a)===T;return b});b.wbg.__wbg_error_71d6845bf00a930f=((b,c)=>{var d=B(b,c).slice();a.__wbindgen_free(b,c*W,W);console.error(...d)});b.wbg.__wbg_body_874ccb42daaab363=(a=>{const b=c(a).body;return y(b)?Q:k(b)});b.wbg.__wbg_createElement_03cf347ddad1c8c0=function(){return C(((a,b,d)=>{const e=c(a).createElement(j(b,d));return k(e)}),arguments)};b.wbg.__wbg_createElementNS_93f8de4acdef6da8=function(){return C(((a,b,d,e,f)=>{const g=c(a).createElementNS(b===Q?L:j(b,d),j(e,f));return k(g)}),arguments)};b.wbg.__wbg_createTextNode_ea32ad2506f7ae78=((a,b,d)=>{const e=c(a).createTextNode(j(b,d));return k(e)});b.wbg.__wbg_instanceof_Window_cee7a886d55e7df5=(a=>{let b;try{b=c(a) instanceof Window}catch(a){b=!1}const d=b;return d});b.wbg.__wbg_document_eb7fd66bde3ee213=(a=>{const b=c(a).document;return y(b)?Q:k(b)});b.wbg.__wbg_instanceof_Element_813f33306edae612=(a=>{let b;try{b=c(a) instanceof Element}catch(a){b=!1}const d=b;return d});b.wbg.__wbg_namespaceURI_230708ae7f4baac5=((b,d)=>{const e=c(d).namespaceURI;var f=y(e)?Q:p(e,a.__wbindgen_malloc,a.__wbindgen_realloc);var g=m;r()[b/W+ R]=g;r()[b/W+ Q]=f});b.wbg.__wbg_setinnerHTML_95222f1a2e797983=((a,b,d)=>{c(a).innerHTML=j(b,d)});b.wbg.__wbg_outerHTML_eb21059e86b1e9f4=((b,d)=>{const e=c(d).outerHTML;const f=p(e,a.__wbindgen_malloc,a.__wbindgen_realloc);const g=m;r()[b/W+ R]=g;r()[b/W+ Q]=f});b.wbg.__wbg_children_ed606b49af931792=(a=>{const b=c(a).children;return k(b)});b.wbg.__wbg_removeAttribute_0c021c98a4dc7402=function(){return C(((a,b,d)=>{c(a).removeAttribute(j(b,d))}),arguments)};b.wbg.__wbg_setAttribute_f7ffa687ef977957=function(){return C(((a,b,d,e,f)=>{c(a).setAttribute(j(b,d),j(e,f))}),arguments)};b.wbg.__wbg_debug_7d82cf3cd21e00b0=(a=>{console.debug(c(a))});b.wbg.__wbg_error_b834525fe62708f5=(a=>{console.error(c(a))});b.wbg.__wbg_info_12174227444ccc71=(a=>{console.info(c(a))});b.wbg.__wbg_log_79d3c56888567995=(a=>{console.log(c(a))});b.wbg.__wbg_warn_2a68e3ab54e55f28=(a=>{console.warn(c(a))});b.wbg.__wbg_instanceof_Event_3282356e36ce8685=(a=>{let b;try{b=c(a) instanceof Event}catch(a){b=!1}const d=b;return d});b.wbg.__wbg_target_6795373f170fd786=(a=>{const b=c(a).target;return y(b)?Q:k(b)});b.wbg.__wbg_bubbles_31126fc08276cf99=(a=>{const b=c(a).bubbles;return b});b.wbg.__wbg_cancelBubble_ae95595adf5ae83d=(a=>{const b=c(a).cancelBubble;return b});b.wbg.__wbg_composedPath_bd8a0336a042e053=(a=>{const b=c(a).composedPath();return k(b)});b.wbg.__wbg_value_ffef403d62e3df58=((b,d)=>{const e=c(d).value;const f=p(e,a.__wbindgen_malloc,a.__wbindgen_realloc);const g=m;r()[b/W+ R]=g;r()[b/W+ Q]=f});b.wbg.__wbg_setvalue_cbab536654d8dd52=((a,b,d)=>{c(a).value=j(b,d)});b.wbg.__wbg_parentNode_e3a5ee563364a613=(a=>{const b=c(a).parentNode;return y(b)?Q:k(b)});b.wbg.__wbg_parentElement_45a9756dc74ff48b=(a=>{const b=c(a).parentElement;return y(b)?Q:k(b)});b.wbg.__wbg_lastChild_d22dbf81f92f163b=(a=>{const b=c(a).lastChild;return y(b)?Q:k(b)});b.wbg.__wbg_nextSibling_87d2b32dfbf09fe3=(a=>{const b=c(a).nextSibling;return y(b)?Q:k(b)});b.wbg.__wbg_setnodeValue_d1cec51282858afe=((a,b,d)=>{c(a).nodeValue=b===Q?L:j(b,d)});b.wbg.__wbg_textContent_528ff517a0418a3e=((b,d)=>{const e=c(d).textContent;var f=y(e)?Q:p(e,a.__wbindgen_malloc,a.__wbindgen_realloc);var g=m;r()[b/W+ R]=g;r()[b/W+ Q]=f});b.wbg.__wbg_appendChild_4153ba1b5d54d73b=function(){return C(((a,b)=>{const d=c(a).appendChild(c(b));return k(d)}),arguments)};b.wbg.__wbg_insertBefore_2be91083083caa9e=function(){return C(((a,b,d)=>{const e=c(a).insertBefore(c(b),c(d));return k(e)}),arguments)};b.wbg.__wbg_removeChild_660924798c7e128c=function(){return C(((a,b)=>{const d=c(a).removeChild(c(b));return k(d)}),arguments)};b.wbg.__wbg_instanceof_ShadowRoot_ef56f954a86c7472=(a=>{let b;try{b=c(a) instanceof ShadowRoot}catch(a){b=!1}const d=b;return d});b.wbg.__wbg_host_dfffc3b2ba786fb8=(a=>{const b=c(a).host;return k(b)});b.wbg.__wbg_addEventListener_bc4a7ad4cc72c6bf=function(){return C(((a,b,d,e,f)=>{c(a).addEventListener(j(b,d),c(e),c(f))}),arguments)};b.wbg.__wbg_instanceof_HtmlInputElement_189f182751dc1f5e=(a=>{let b;try{b=c(a) instanceof HTMLInputElement}catch(a){b=!1}const d=b;return d});b.wbg.__wbg_setchecked_50e21357d62a8ccd=((a,b)=>{c(a).checked=b!==Q});b.wbg.__wbg_value_99f5294791d62576=((b,d)=>{const e=c(d).value;const f=p(e,a.__wbindgen_malloc,a.__wbindgen_realloc);const g=m;r()[b/W+ R]=g;r()[b/W+ Q]=f});b.wbg.__wbg_setvalue_bba31de32cdbb32c=((a,b,d)=>{c(a).value=j(b,d)});b.wbg.__wbg_get_0ee8ea3c7c984c45=((a,b)=>{const d=c(a)[b>>>Q];return k(d)});b.wbg.__wbg_length_161c0d89c6535c1d=(a=>{const b=c(a).length;return b});b.wbg.__wbg_newnoargs_cfecb3965268594c=((a,b)=>{const c=new Function(j(a,b));return k(c)});b.wbg.__wbg_call_3f093dd26d5569f8=function(){return C(((a,b)=>{const d=c(a).call(c(b));return k(d)}),arguments)};b.wbg.__wbg_new_632630b5cec17f21=(()=>{const a=new X();return k(a)});b.wbg.__wbg_self_05040bd9523805b9=function(){return C((()=>{const a=self.self;return k(a)}),arguments)};b.wbg.__wbg_window_adc720039f2cb14f=function(){return C((()=>{const a=window.window;return k(a)}),arguments)};b.wbg.__wbg_globalThis_622105db80c1457d=function(){return C((()=>{const a=globalThis.globalThis;return k(a)}),arguments)};b.wbg.__wbg_global_f56b013ed9bcf359=function(){return C((()=>{const a=global.global;return k(a)}),arguments)};b.wbg.__wbindgen_is_undefined=(a=>{const b=c(a)===L;return b});b.wbg.__wbg_from_58c79ccfb68060f5=(a=>{const b=J.from(c(a));return k(b)});b.wbg.__wbg_is_bd5dc4ae269cba1c=((a,b)=>{const d=X.is(c(a),c(b));return d});b.wbg.__wbg_resolve_5da6faf2c96fd1d5=(a=>{const b=Promise.resolve(c(a));return k(b)});b.wbg.__wbg_then_f9e58f5a50f43eae=((a,b)=>{const d=c(a).then(c(b));return k(d)});b.wbg.__wbg_set_961700853a212a39=function(){return C(((a,b,d)=>{const e=Reflect.set(c(a),c(b),c(d));return e}),arguments)};b.wbg.__wbindgen_debug_string=((b,d)=>{const e=l(c(d));const f=p(e,a.__wbindgen_malloc,a.__wbindgen_realloc);const g=m;r()[b/W+ R]=g;r()[b/W+ Q]=f});b.wbg.__wbindgen_throw=((a,b)=>{throw new P(j(a,b))});b.wbg.__wbindgen_closure_wrapper656=((a,b,c)=>{const d=t(a,b,251,w);return k(d)});b.wbg.__wbindgen_closure_wrapper778=((a,b,c)=>{const d=t(a,b,299,x);return k(d)});return b});var A=(()=>{if(z===M||z.byteLength===Q){z=new Uint32Array(a.memory.buffer)};return z});var x=((b,c,d)=>{a._dyn_core__ops__function__FnMut__A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__hcbcb2fe54ceab896(b,c,k(d))});var e=(a=>{if(a<132)return;b[a]=d;d=a});var t=((b,c,d,e)=>{const f={a:b,b:c,cnt:R,dtor:d};const g=(...b)=>{f.cnt++;const c=f.a;f.a=Q;try{return e(c,f.b,...b)}finally{if(--f.cnt===Q){a.__wbindgen_export_2.get(f.dtor)(c,f.b);s.unregister(f)}else{f.a=c}}};g.original=f;s.register(g,f,f);return g});var p=((a,b,c)=>{if(c===L){const c=n.encode(a);const d=b(c.length,R)>>>Q;i().subarray(d,d+ c.length).set(c);m=c.length;return d};let d=a.length;let e=b(d,R)>>>Q;const f=i();let g=Q;for(;g<d;g++){const b=a.charCodeAt(g);if(b>127)break;f[e+ g]=b};if(g!==d){if(g!==Q){a=a.slice(g)};e=c(e,d,d=g+ a.length*3,R)>>>Q;const b=i().subarray(e+ g,e+ d);const f=o(a,b);g+=f.written;e=c(e,d,g,R)>>>Q};m=g;return e});var i=(()=>{if(h===M||h.byteLength===Q){h=new Uint8Array(a.memory.buffer)};return h});var j=((a,b)=>{a=a>>>Q;return g.decode(i().subarray(a,a+ b))});var v=(a=>{if(u==R)throw new P(`out of js stack`);b[--u]=a;return u});let a;const b=new J(K).fill(L);b.push(L,M,!0,!1);let d=b.length;const g=typeof TextDecoder!==N?new TextDecoder(O,{ignoreBOM:!0,fatal:!0}):{decode:()=>{throw P(`TextDecoder not available`)}};if(typeof TextDecoder!==N){g.decode()};let h=M;let m=Q;const n=typeof TextEncoder!==N?new TextEncoder(O):{encode:()=>{throw P(`TextEncoder not available`)}};const o=typeof n.encodeInto===T?((a,b)=>n.encodeInto(a,b)):((a,b)=>{const c=n.encode(a);b.set(c);return {read:a.length,written:c.length}});let q=M;const s=typeof V===N?{register:()=>{},unregister:()=>{}}:new V(b=>{a.__wbindgen_export_2.get(b.dtor)(b.a,b.b)});let u=K;let z=M;export default I;export{H as initSync}