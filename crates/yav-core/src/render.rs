//! HTML rendering engine.
//!
//! Converts a [`Node`] tree into a valid HTML string with proper escaping,
//! attribute serialization, and void element handling.

use crate::escape::{escape_attribute, escape_html};
use crate::node::{AttributeValue, Node};

/// Render a [`Node`] tree to an HTML string.
///
/// All text content is automatically HTML-escaped. Attribute values are
/// attribute-escaped. Void elements are self-closed. Boolean attributes
/// that are `false` are omitted.
///
/// # Examples
/// ```
/// use yav_core::{Element, Node, render_to_string};
///
/// let node = Node::Element(
///     Element::new("div")
///         .attr("class", "container")
///         .child(Node::text("Hello, world!"))
/// );
/// assert_eq!(render_to_string(&node), r#"<div class="container">Hello, world!</div>"#);
/// ```
pub fn render_to_string(node: &Node) -> String {
    let capacity = estimate_node_size(node).min(1024 * 1024);
    let mut buf = String::with_capacity(capacity);
    render_node(node, &mut buf);
    buf
}

/// Estimate the rendered byte size of a node tree for pre-allocation.
fn estimate_node_size(node: &Node) -> usize {
    match node {
        Node::Text(t) => t.len() + 8, // possible escaping overhead
        Node::Raw(r) => r.len(),
        Node::Static(s) => s.len(),
        Node::Element(el) => {
            let tag_overhead = el.tag.len() * 2 + 5; // <tag></tag>
            let attr_size: usize = el
                .attributes
                .iter()
                .map(|a| a.name.len() + 10) // name="value" estimate
                .sum();
            let children_size: usize = el.children.iter().map(estimate_node_size).sum();
            tag_overhead + attr_size + children_size
        }
        Node::Fragment(children) => children.iter().map(estimate_node_size).sum(),
        Node::Empty => 0,
    }
}

const MAX_RENDER_DEPTH: usize = 512;

/// Render a [`Node`] tree into an existing buffer.
pub fn render_to_buf(node: &Node, buf: &mut String) {
    render_node_depth(node, buf, 0);
}

fn render_node(node: &Node, buf: &mut String) {
    render_node_depth(node, buf, 0);
}

fn render_node_depth(node: &Node, buf: &mut String, depth: usize) {
    if depth > MAX_RENDER_DEPTH {
        buf.push_str("<!-- yav: render depth limit exceeded -->");
        return;
    }
    match node {
        Node::Element(el) => render_element_depth(el, buf, depth + 1),
        Node::Text(text) => buf.push_str(&escape_html(text)),
        Node::Raw(html) => buf.push_str(html),
        Node::Static(s) => buf.push_str(s),
        Node::Fragment(children) => {
            for child in children {
                render_node_depth(child, buf, depth + 1);
            }
        }
        Node::Empty => {}
    }
}

fn is_url_attribute(name: &str) -> bool {
    const URL_ATTRS: &[&str] = &[
        "href",
        "src",
        "action",
        "formaction",
        "xlink:href",
        "poster",
        "data",
        "cite",
        "background",
        "ping",
        "form",
        "usemap",
    ];
    URL_ATTRS.iter().any(|attr| name.eq_ignore_ascii_case(attr))
}

fn is_valid_tag_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    let mut chars = name.chars();
    let first = match chars.next() {
        Some(c) => c,
        None => return false,
    };
    if !first.is_ascii_alphabetic() {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == ':')
}

fn is_valid_attribute_name(name: &str) -> bool {
    !name.is_empty()
        && !name.chars().any(|c| {
            c.is_ascii_whitespace()
                || c.is_control()
                || c == '"'
                || c == '\''
                || c == '>'
                || c == '/'
                || c == '='
        })
}

// Render a dynamic `AttributeValue::String` with context-aware escaping.
// `srcdoc`, `srcset` and `style` are not plain URLs and need dedicated helpers;
// URL attributes keep going through `escape_url`. A rejected srcset/URL simply
// yields nothing, so the attribute is omitted. Static attributes are untouched.
fn render_dynamic_string_attribute(name: &str, val: &str, buf: &mut String) {
    let safe: Option<String> = if name.eq_ignore_ascii_case("srcdoc") {
        Some(crate::escape::escape_srcdoc(val))
    } else if name.eq_ignore_ascii_case("srcset") {
        crate::escape::escape_srcset(val)
    } else if name.eq_ignore_ascii_case("style") {
        Some(crate::escape::escape_style(val))
    } else if is_url_attribute(name) {
        crate::escape::escape_url(val)
    } else {
        Some(escape_attribute(val).into_owned())
    };

    if let Some(safe) = safe {
        buf.push(' ');
        buf.push_str(name);
        buf.push_str("=\"");
        buf.push_str(&safe);
        buf.push('"');
    }
}

fn render_element_depth(el: &crate::node::Element, buf: &mut String, depth: usize) {
    let safe_tag = if is_valid_tag_name(&el.tag) {
        el.tag.as_ref()
    } else {
        "div"
    };

    // Opening tag
    buf.push('<');
    buf.push_str(safe_tag);

    // Attributes
    for attr in &el.attributes {
        if !is_valid_attribute_name(&attr.name) {
            continue;
        }
        match &attr.value {
            AttributeValue::String(val) => {
                render_dynamic_string_attribute(&attr.name, val, buf);
            }
            AttributeValue::Static(val) => {
                render_dynamic_string_attribute(&attr.name, val, buf);
            }
            AttributeValue::Bool(true) => {
                buf.push(' ');
                buf.push_str(&attr.name);
            }
            AttributeValue::Bool(false) => {
                // Omit false boolean attributes entirely
            }
            AttributeValue::None => {
                buf.push(' ');
                buf.push_str(&attr.name);
            }
        }
    }

    if el.self_closing && crate::node::is_void_element(safe_tag) {
        buf.push_str(" />");
        return;
    }

    buf.push('>');

    // Children
    let is_script = safe_tag.eq_ignore_ascii_case("script");
    let is_style = safe_tag.eq_ignore_ascii_case("style");

    for child in &el.children {
        if is_script {
            render_rawtext_child(child, buf, "</script", "<\\/script", depth + 1);
        } else if is_style {
            render_rawtext_child(child, buf, "</style", "<\\/style", depth + 1);
        } else {
            render_node_depth(child, buf, depth + 1);
        }
    }

    // Closing tag
    buf.push_str("</");
    buf.push_str(safe_tag);
    buf.push('>');
}

fn replace_case_insensitive(input: &str, target: &str, replacement: &str) -> String {
    let lower_input = input.to_ascii_lowercase();
    let lower_target = target.to_ascii_lowercase();
    if !lower_input.contains(&lower_target) {
        return input.to_string();
    }
    let mut result = String::with_capacity(input.len());
    let mut last = 0;
    for (idx, _) in lower_input.match_indices(&lower_target) {
        result.push_str(&input[last..idx]);
        result.push_str(replacement);
        last = idx + target.len();
    }
    result.push_str(&input[last..]);
    result
}

fn render_rawtext_child(
    node: &Node,
    buf: &mut String,
    close_tag: &str,
    replacement: &str,
    depth: usize,
) {
    if depth > MAX_RENDER_DEPTH {
        buf.push_str("/* yav: render depth limit exceeded */");
        return;
    }
    match node {
        Node::Text(text) => {
            let sanitized = replace_case_insensitive(text, close_tag, replacement);
            buf.push_str(&sanitized);
        }
        Node::Raw(raw) => {
            let sanitized = replace_case_insensitive(raw, close_tag, replacement);
            buf.push_str(&sanitized);
        }
        Node::Static(s) => {
            let sanitized = replace_case_insensitive(s, close_tag, replacement);
            buf.push_str(&sanitized);
        }
        Node::Fragment(children) => {
            for child in children {
                render_rawtext_child(child, buf, close_tag, replacement, depth + 1);
            }
        }
        Node::Element(el) => {
            let mut inner = String::new();
            render_element_depth(el, &mut inner, depth + 1);
            let sanitized = replace_case_insensitive(&inner, close_tag, replacement);
            buf.push_str(&sanitized);
        }
        Node::Empty => {}
    }
}

/// Ultra-compact, dependency-free client micro-actions runtime with non-destructive DOM Morphing.
///
/// Handles:
/// - `yav:toggle`, `yav:show`, `yav:hide`, `yav:click-outside="close"`, `yav:escape="close"`, `yav:group`, `yav:close-on-resize`
/// - Declarative AJAX mutations: `yav:get`, `yav:post`, `yav:put`, `yav:delete` with `yav:target` and `yav:swap` (`morph`, `innerHTML`, `outerHTML`, `beforeend`, `afterbegin`)
/// - Non-destructive DOM morphing (`morphEl`): preserves `[yav:preserve]`, focused `<input>`/`<textarea>` values & selection ranges
/// - SPA-like navigation & prefetching: `yav:boost` (with View Transitions API & DOM morphing) and `yav:prefetch="hover"`
/// - Native clipboard copy & toast feedback: `yav:copy`, `yav:copy-label`, `yav:toast`
/// - Multi-pane synchronized tabs: `yav:tab-group`, `yav:tab`, `yav:tab-panel`
/// - Instant client-side filtering: `yav:filter`, `yav:filter-empty`, `yav:filter-target`, `yav:filter-attr`, `yav:filter-value`
/// - Client-side i18n switching: `yav:set-lang`, `#yav-i18n-payload`, `[data-i18n]`
/// - Active nav link matching: `yav:nav-active`, `[data-nav]`, and global hotkeys `yav:hotkey`
/// - WAI-ARIA modal dialog focus traps and roving arrow-key menu navigation
///
/// ## Extended UI component actions (CSP-safe, delegated)
///
/// The companion `YAV_UI_ACTIONS_SCRIPT` handles component-level actions that
/// cannot be expressed with core `yav:*` attributes alone:
/// - `[data-dismiss="toast"]` – remove the closest `[data-toast]` ancestor
/// - `[data-theme-toggle]` – toggle `dark` class on `<html>` and persist to `localStorage`
/// - `[data-copy-code]` – copy the text content of the nearest code block (strips line-number spans)
/// - `[data-seek-seconds]` – seek the nearest `<video>` to the specified second and play
pub const YAV_MICRO_ACTIONS_SCRIPT: &str = r##"<script id="yav-micro-actions">(function(){function g(e){var s=e.getAttribute('yav:toggle')||e.getAttribute('data-yav-toggle')||e.getAttribute('yav:show')||e.getAttribute('data-yav-show')||e.getAttribute('yav:hide')||e.getAttribute('data-yav-hide');return s?document.querySelector(s):null}function c(t,r){if(!t)return;t.classList.add('hidden');t.setAttribute('hidden','');t.removeAttribute('data-open');t.classList.remove('is-open');var trig=r;if(!trig&&t.id){trig=document.querySelector('[yav\\:toggle="#'+t.id+'"],[data-yav-toggle="#'+t.id+'"]')}if(trig){trig.setAttribute('aria-expanded','false');if(document.activeElement&&t.contains(document.activeElement)){trig.focus()}}}function o(t,r){if(!t)return;var grp=t.getAttribute('yav:group')||t.getAttribute('data-yav-group');if(grp){document.querySelectorAll('[yav\\:group="'+grp+'"],[data-yav-group="'+grp+'"]').forEach(function(other){if(other!==t)c(other)})}t.classList.remove('hidden');t.removeAttribute('hidden');t.setAttribute('data-open','true');t.classList.add('is-open');if(r)r.setAttribute('aria-expanded','true');var f=t.querySelector('[autofocus],input,button,a[href],[tabindex]:not([tabindex="-1"])');if(f)setTimeout(function(){f.focus()},10)}function sp(){var b=document.getElementById('yav-top-loader');if(!b){b=document.createElement('div');b.id='yav-top-loader';b.style.cssText='position:fixed;top:0;left:0;height:2.5px;width:0%;background:linear-gradient(90deg,#6366f1,#06b6d4);z-index:99999;transition:width .2s ease,opacity .3s ease;pointer-events:none';document.body.appendChild(b)}b.style.opacity='1';b.style.width='35%';setTimeout(function(){if(b.style.opacity==='1')b.style.width='78%'},100)}function hp(){var b=document.getElementById('yav-top-loader');if(b){b.style.width='100%';setTimeout(function(){b.style.opacity='0';b.style.width='0%'},220)}}function toast(msg){if(!msg)return;var t=document.getElementById('cmd-notification')||document.querySelector('[data-yav-toast]');if(!t){t=document.createElement('div');t.setAttribute('data-yav-toast','');t.style.cssText='position:fixed;bottom:24px;right:24px;z-index:9999;padding:10px 16px;background:#111112;color:#ece9e3;border:1px solid rgba(236,233,227,0.16);font-family:monospace;font-size:12px;transition:all .25s ease;opacity:0;transform:translateY(12px);pointer-events:none';document.body.appendChild(t)}var tx=t.querySelector('#cmd-notification-text,[data-yav-toast-text]')||t;tx.textContent=msg;t.classList.remove('translate-y-12','opacity-0');t.style.opacity='1';t.style.transform='translateY(0)';clearTimeout(t.__yav_tm);t.__yav_tm=setTimeout(function(){t.style.opacity='0';t.style.transform='translateY(12px)';t.classList.add('translate-y-12','opacity-0')},2000)}function applyLang(lang){if(!lang)return;try{localStorage.setItem('yav_lang',lang);document.cookie='yav_lang='+lang+'; path=/; max-age=31536000'}catch(_){}var lbl=document.getElementById('lang-current-label')||document.querySelector('[data-yav-lang-label]');if(lbl)lbl.textContent=lang.toUpperCase();document.documentElement.lang=lang;var sc=document.getElementById('yav-i18n-payload');if(!sc)return;try{var d=JSON.parse(sc.textContent||'{}');var tr=d[lang]||d['en'];if(!tr)return;document.querySelectorAll('[data-i18n]').forEach(function(el){var k=el.getAttribute('data-i18n');if(tr[k])el.textContent=tr[k]});document.querySelectorAll('[data-i18n-placeholder]').forEach(function(el){var k=el.getAttribute('data-i18n-placeholder');if(tr[k])el.placeholder=tr[k]})}catch(_){}}function syncNav(){var p=location.pathname.replace(/\/$/,'')||'/';document.querySelectorAll('[yav\\:nav-active],[data-yav-nav-active],[data-nav]').forEach(function(a){var h=(a.getAttribute('href')||'').replace(/\/$/,'')||'/';var v=a.getAttribute('yav:nav-active')||a.getAttribute('data-yav-nav-active')||'page';if(h===p)a.setAttribute('aria-current',v);else a.removeAttribute('aria-current')})}function morphEl(from,to){if(!from||!to)return;if(from.nodeType===1&&(from.hasAttribute('yav:preserve')||from.hasAttribute('data-yav-preserve')))return;if(from.nodeType!==to.nodeType||from.nodeName!==to.nodeName){from.replaceWith(to.cloneNode(true));return}if(from.nodeType===3||from.nodeType===8){if(from.nodeValue!==to.nodeValue)from.nodeValue=to.nodeValue;return}if(from.nodeType===1){var isAct=document.activeElement===from&&(from.tagName==='INPUT'||from.tagName==='TEXTAREA'||from.tagName==='SELECT');var oldAttrs=from.attributes,newAttrs=to.attributes;for(var i=oldAttrs.length-1;i>=0;i--){var nm=oldAttrs[i].name;if(!to.hasAttribute(nm))from.removeAttribute(nm)}for(var j=0;j<newAttrs.length;j++){var a=newAttrs[j];if(isAct&&a.name==='value')continue;if(from.getAttribute(a.name)!==a.value)from.setAttribute(a.name,a.value)}if(!isAct&&'value' in to&&from.value!==to.value)from.value=to.value;var fc=Array.from(from.childNodes),tc=Array.from(to.childNodes),max=Math.max(fc.length,tc.length);for(var k=0;k<max;k++){if(k>=tc.length){if(fc[k]&&!(fc[k].nodeType===1&&(fc[k].hasAttribute('yav:preserve')||fc[k].hasAttribute('data-yav-preserve'))))fc[k].remove()}else if(k>=fc.length){from.appendChild(tc[k].cloneNode(true))}else{morphEl(fc[k],tc[k])}}}}window.__yavMorph=morphEl;function sw(el,html,mode){if(!el)return;if(mode==='outerHTML')el.outerHTML=html;else if(mode==='beforeend')el.insertAdjacentHTML('beforeend',html);else if(mode==='afterbegin')el.insertAdjacentHTML('afterbegin',html);else if(mode==='delete')el.remove();else if(mode==='morph'){var tmp=document.createElement(el.tagName||'div');tmp.innerHTML=html;morphEl(el,tmp)}else el.innerHTML=html;window.dispatchEvent(new CustomEvent('yav:navigate'))}function ax(el,m,url,body){var ts=el.getAttribute('yav:target')||el.getAttribute('data-yav-target');var tg=ts?document.querySelector(ts):el;var sm=el.getAttribute('yav:swap')||el.getAttribute('data-yav-swap')||'innerHTML';var optTog=el.getAttribute('yav:optimistic-toggle')||el.getAttribute('data-yav-optimistic-toggle');var optInc=el.getAttribute('yav:optimistic-inc')||el.getAttribute('data-yav-optimistic-inc');var optSet=el.getAttribute('yav:optimistic-set')||el.getAttribute('data-yav-optimistic-set');var rbMsg=el.getAttribute('yav:rollback-toast')||el.getAttribute('data-yav-rollback-toast');var prevTog=optTog?el.classList.contains(optTog):null;var incEl=optInc?document.querySelector(optInc):null;var prevInc=incEl?incEl.textContent:null;var setEl=null,prevSet=null;if(optSet){var sp=optSet.split(':');setEl=document.querySelector(sp[0]);if(setEl){prevSet=setEl.textContent;setEl.textContent=sp.slice(1).join(':')}}if(optTog)el.classList.toggle(optTog);if(incEl){var iv=parseInt(incEl.textContent||'0',10);incEl.textContent=isNaN(iv)?'1':(iv+1)}fetch(url,{method:m,body:body,headers:{'X-Yav-Request':'true','HX-Request':'true'}}).then(function(r){if(!r.ok)throw new Error(r.statusText);return r.text()}).then(function(h){sw(tg,h,sm)}).catch(function(){if(optTog!==null){if(prevTog)el.classList.add(optTog);else el.classList.remove(optTog)}if(incEl&&prevInc!==null)incEl.textContent=prevInc;if(setEl&&prevSet!==null)setEl.textContent=prevSet;if(rbMsg)toast(rbMsg)})}var j=null;document.addEventListener('click',function(e){var cp=e.target.closest('[yav\\:copy],[data-yav-copy]');if(cp){var txt=cp.getAttribute('yav:copy')||cp.getAttribute('data-yav-copy');if(txt&&navigator.clipboard){navigator.clipboard.writeText(txt);var lb=cp.querySelector('[data-copy-label]')||cp;var fb=cp.getAttribute('yav:copy-label')||cp.getAttribute('data-yav-copy-label')||'copied';var prev=lb.getAttribute('data-prev-text')||lb.textContent;lb.setAttribute('data-prev-text',prev);lb.textContent=fb;setTimeout(function(){lb.textContent=prev},1400);var tm=cp.getAttribute('yav:toast')||cp.getAttribute('data-yav-toast');if(tm)toast(tm)}}var lg=e.target.closest('[yav\\:set-lang],[data-yav-set-lang]');if(lg){var l=lg.getAttribute('yav:set-lang')||lg.getAttribute('data-yav-set-lang');applyLang(l);var pop=lg.closest('[data-open="true"]');if(pop)c(pop);return}var tb=e.target.closest('[yav\\:tab],[data-yav-tab]');if(tb){var grp=tb.getAttribute('yav:tab-group')||tb.getAttribute('data-yav-tab-group')||'default';var val=tb.getAttribute('yav:tab')||tb.getAttribute('data-yav-tab');document.querySelectorAll('[yav\\:tab-group="'+grp+'"],[data-yav-tab-group="'+grp+'"]').forEach(function(b){var act=(b.getAttribute('yav:tab')||b.getAttribute('data-yav-tab'))===val;b.classList.toggle('active',act);b.setAttribute('aria-selected',act?'true':'false')});document.querySelectorAll('[yav\\:tab-panel^="'+grp+':"],[data-yav-tab-panel^="'+grp+':"]').forEach(function(p){var pv=p.getAttribute('yav:tab-panel')||p.getAttribute('data-yav-tab-panel');var match=pv===(grp+':'+val);p.classList.toggle('hidden',!match);if(match)window.dispatchEvent(new CustomEvent('yav:tab-change',{detail:{group:grp,tab:val,panel:p}}))});return}var fc=e.target.closest('[yav\\:filter-value],[data-yav-filter-value]');if(fc){var ft=fc.getAttribute('yav:filter-target')||fc.getAttribute('data-yav-filter-target');var fa=fc.getAttribute('yav:filter-attr')||fc.getAttribute('data-yav-filter-attr')||'data-cat';var fv=fc.getAttribute('yav:filter-value')||fc.getAttribute('data-yav-filter-value')||'all';if(ft){document.querySelectorAll(ft).forEach(function(it){var iv=it.getAttribute(fa)||'';it.style.display=(fv==='all'||iv===fv)?'':'none'})}return}var t=e.target.closest('[yav\\:toggle],[data-yav-toggle],[yav\\:show],[data-yav-show],[yav\\:hide],[data-yav-hide]');if(t){var tg=g(t);if(tg){document.querySelectorAll('[yav\\:click-outside="close"],[data-yav-click-outside="close"]').forEach(function(el){if(el!==tg&&!el.contains(t)&&!el.contains(tg)){var open=el.getAttribute('data-open')==='true'||(!el.classList.contains('hidden')&&!el.hasAttribute('hidden'));if(open)c(el)}});if(t.hasAttribute('yav:show')||t.hasAttribute('data-yav-show'))o(tg,t);else if(t.hasAttribute('yav:hide')||t.hasAttribute('data-yav-hide'))c(tg,t);else{(tg.getAttribute('data-open')==='true'||(!tg.classList.contains('hidden')&&!tg.hasAttribute('hidden')))?c(tg,t):o(tg,t)}j=tg;return}}var xm=e.target.closest('[yav\\:get],[data-yav-get],[yav\\:post],[data-yav-post],[yav\\:put],[data-yav-put],[yav\\:delete],[data-yav-delete]');if(xm&&xm.tagName!=='FORM'){e.preventDefault();var m='GET',u=xm.getAttribute('yav:get')||xm.getAttribute('data-yav-get');if(!u){m='POST';u=xm.getAttribute('yav:post')||xm.getAttribute('data-yav-post')}if(!u){m='PUT';u=xm.getAttribute('yav:put')||xm.getAttribute('data-yav-put')}if(!u){m='DELETE';u=xm.getAttribute('yav:delete')||xm.getAttribute('data-yav-delete')}if(u)ax(xm,m,u,null);return}var a=e.target.closest('a[href]');if(a&&!e.ctrlKey&&!e.metaKey&&!e.shiftKey&&a.target!=='_blank'){var href=a.getAttribute('href');if(href&&href.charAt(0)==='/'&&(a.hasAttribute('yav:boost')||a.closest('[yav\\:boost],[data-yav-boost]'))){e.preventDefault();sp();fetch(href,{headers:{'X-Yav-Boost':'true'}}).then(function(r){return r.text()}).then(function(html){hp();var apply=function(){var doc=new DOMParser().parseFromString(html,'text/html');if(doc.title)document.title=doc.title;if(doc.body)morphEl(document.body,doc.body);history.pushState({},'',href);window.scrollTo(0,0);syncNav();window.dispatchEvent(new CustomEvent('yav:navigate'))};if(document.startViewTransition)document.startViewTransition(apply);else apply()}).catch(function(){hp();window.location.href=href});return}}document.querySelectorAll('[yav\\:click-outside="close"],[data-yav-click-outside="close"]').forEach(function(el){if(el===j)return;var open=el.getAttribute('data-open')==='true'||(!el.classList.contains('hidden')&&!el.hasAttribute('hidden'));if(open&&!el.contains(e.target))c(el)});j=null});document.addEventListener('input',function(e){var fi=e.target.closest('[yav\\:filter],[data-yav-filter]');if(fi){var sel=fi.getAttribute('yav:filter')||fi.getAttribute('data-yav-filter');var emp=fi.getAttribute('yav:filter-empty')||fi.getAttribute('data-yav-filter-empty');var q=(fi.value||'').toLowerCase().trim();var vis=0;if(sel){document.querySelectorAll(sel).forEach(function(it){var tx=((it.textContent||'')+' '+(it.getAttribute('data-name')||'')).toLowerCase();var ok=!q||tx.indexOf(q)!==-1;it.style.display=ok?'':'none';if(ok)vis++})}if(emp){var emEl=document.querySelector(emp);if(emEl)emEl.classList.toggle('hidden',vis>0)}}});document.addEventListener('submit',function(e){var f=e.target.closest('form[yav\\:post],form[data-yav-post],form[yav\\:get],form[data-yav-get],form[yav\\:put],form[data-yav-put],form[yav\\:delete],form[data-yav-delete]');if(f){e.preventDefault();var m='POST',u=f.getAttribute('yav:post')||f.getAttribute('data-yav-post');if(!u){m='GET';u=f.getAttribute('yav:get')||f.getAttribute('data-yav-get')}if(!u){m='PUT';u=f.getAttribute('yav:put')||f.getAttribute('data-yav-put')}if(!u){m='DELETE';u=f.getAttribute('yav:delete')||f.getAttribute('data-yav-delete')}if(u)ax(f,m,u,new URLSearchParams(new FormData(f)))}});var pf={};document.addEventListener('mouseover',function(e){var a=e.target.closest('a[href]');if(a&&(a.getAttribute('yav:prefetch')==='hover'||a.getAttribute('data-yav-prefetch')==='hover')){var h=a.getAttribute('href');if(h&&h.charAt(0)==='/'&&!pf[h]){pf[h]=1;var l=document.createElement('link');l.rel='prefetch';l.href=h;document.head.appendChild(l)}}});document.addEventListener('keydown',function(e){if(e.key==='Escape'){document.querySelectorAll('[yav\\:escape="close"],[data-yav-escape="close"],[yav\\:click-outside="close"],[data-yav-click-outside="close"]').forEach(function(el){c(el)});return}if((e.ctrlKey||e.metaKey)&&e.key){var k=e.key.toLowerCase();var hk=document.querySelector('[yav\\:hotkey*="+'+k+'"],[data-yav-hotkey*="+'+k+'"]');if(hk){e.preventDefault();hk.click();return}}var d=document.querySelector('dialog[open],[role="dialog"][data-open="true"],[yav\\:focus-trap][data-open="true"]');if(d&&e.key==='Tab'){var fs=Array.from(d.querySelectorAll('a[href],button:not([disabled]),textarea:not([disabled]),input:not([disabled]),select:not([disabled]),[tabindex]:not([tabindex="-1"])'));if(fs.length>0){var f0=fs[0],f1=fs[fs.length-1];if(e.shiftKey&&document.activeElement===f0){f1.focus();e.preventDefault()}else if(!e.shiftKey&&document.activeElement===f1){f0.focus();e.preventDefault()}}}var m=document.activeElement?document.activeElement.closest('[role="menu"]'):null;if(m){var items=Array.from(m.querySelectorAll('[role="menuitem"],a,button'));var idx=items.indexOf(document.activeElement);if(e.key==='ArrowDown'){e.preventDefault();var next=items[(idx+1)%items.length];if(next)next.focus()}else if(e.key==='ArrowUp'){e.preventDefault();var prev=items[(idx-1+items.length)%items.length];if(prev)prev.focus()}else if(e.key==='Home'){e.preventDefault();if(items[0])items[0].focus()}else if(e.key==='End'){e.preventDefault();if(items[items.length-1])items[items.length-1].focus()}}});window.addEventListener('resize',function(){document.querySelectorAll('[yav\\:close-on-resize],[data-yav-close-on-resize]').forEach(function(el){var w=parseInt(el.getAttribute('yav:close-on-resize')||el.getAttribute('data-yav-close-on-resize')||'1024',10);if(window.innerWidth>=w)c(el)})});window.addEventListener('yav:navigate',hp);syncNav();try{var sl=localStorage.getItem('yav_lang');if(sl)applyLang(sl)}catch(_){}})();</script>"##;

/// CSP-safe delegated handlers for Yav UI components that cannot be expressed
/// with core `yav:*` attributes alone. Always tree-shaken in alongside the
/// micro-actions script; uses only `addEventListener` (no inline event handlers).
///
/// Handles:
/// - `[data-dismiss="toast"]` – remove the closest `[data-toast]` ancestor
/// - `[data-theme-toggle]`   – toggle `dark` on `<html>`, persist to `localStorage`
/// - `[data-copy-code]`      – copy code block text (stripping line-number spans)
/// - `[data-seek-seconds]`   – seek nearest `<video>` to N seconds and play
pub const YAV_UI_ACTIONS_SCRIPT: &str = r##"<script id="yav-ui-actions">(function(){document.addEventListener('click',function(e){var dm=e.target.closest('[data-dismiss]');if(dm){var kind=dm.getAttribute('data-dismiss');if(kind==='toast'){var t=dm.closest('[data-toast]');if(t)t.remove()}return}var tt=e.target.closest('[data-theme-toggle]');if(tt){var d=document.documentElement.classList.toggle('dark');localStorage.setItem('theme',d?'dark':'light');return}var cc=e.target.closest('[data-copy-code]');if(cc){var bl=cc.closest('.yav-codeblock');if(bl){var code=bl.querySelector('code');if(code){var cl=code.cloneNode(true);cl.querySelectorAll('[data-line-num]').forEach(function(n){n.remove()});navigator.clipboard.writeText(cl.innerText);cc.textContent='Copied!';setTimeout(function(){cc.textContent='Copy'},2000)}}return}var sk=e.target.closest('[data-seek-seconds]');if(sk){var sec=parseFloat(sk.getAttribute('data-seek-seconds'));var mp=sk.closest('.yav-media-player');var v=mp?mp.querySelector('video'):null;if(v&&!isNaN(sec)){v.currentTime=sec;v.play()}return}})})();</script>"##;

/// Modular cursor & spatial physics runtime (`yav:cursor="vars|spotlight|tilt|magnetic|follower"`, `yav:cursor-global`).
/// Auto-injected by SSR tree-shaking only when cursor attributes are present in the document.
pub const YAV_CURSOR_RUNTIME_SCRIPT: &str = r##"<script id="yav-cursor-runtime">(function(){if(window.__YAV_CURSOR__)return;window.__YAV_CURSOR__=1;var px=0,py=0,raf=0,fl=[];function tick(){raf=0;if(document.documentElement.hasAttribute('yav:cursor-global')||document.body.hasAttribute('yav:cursor-global')){document.documentElement.style.setProperty('--yav-page-x',px+'px');document.documentElement.style.setProperty('--yav-page-y',py+'px')}var els=document.querySelectorAll('[yav\\:cursor],[data-yav-cursor]');els.forEach(function(el){var mode=el.getAttribute('yav:cursor')||el.getAttribute('data-yav-cursor')||'vars';if(mode==='follower')return;var r=el.getBoundingClientRect();if(px<r.left-120||px>r.right+120||py<r.top-120||py>r.bottom+120)return;var x=px-r.left,y=py-r.top,w=r.width||1,h=r.height||1;var rx=Math.max(0,Math.min(1,x/w)),ry=Math.max(0,Math.min(1,y/h));var cx=(rx-0.5)*2,cy=(ry-0.5)*2;el.style.setProperty('--yav-mouse-x',x.toFixed(1)+'px');el.style.setProperty('--yav-mouse-y',y.toFixed(1)+'px');el.style.setProperty('--yav-mouse-rx',rx.toFixed(3));el.style.setProperty('--yav-mouse-ry',ry.toFixed(3));el.style.setProperty('--yav-mouse-cx',cx.toFixed(3));el.style.setProperty('--yav-mouse-cy',cy.toFixed(3));var inside=x>=0&&x<=w&&y>=0&&y<=h;if(mode.indexOf('spotlight')!==-1){el.style.setProperty('--yav-spotlight-opacity',inside?'1':'0')}if(mode.indexOf('tilt')!==-1){if(inside){var max=parseFloat(el.getAttribute('yav:tilt-max')||el.getAttribute('data-yav-tilt-max')||'8');var persp=el.getAttribute('yav:tilt-perspective')||'800px';var tx=(-cy*max).toFixed(2),ty=(cx*max).toFixed(2);el.style.transition='transform 80ms linear';el.style.transform='perspective('+persp+') rotateX('+tx+'deg) rotateY('+ty+'deg) scale3d(1.01,1.01,1.01)'}else if(el.__yav_tilted){el.__yav_tilted=0;el.style.transition='transform 420ms cubic-bezier(0.2,0.8,0.2,1)';el.style.transform='perspective(800px) rotateX(0deg) rotateY(0deg) scale3d(1,1,1)'}if(inside)el.__yav_tilted=1}if(mode.indexOf('magnetic')!==-1){if(inside){var str=parseFloat(el.getAttribute('yav:magnetic-strength')||el.getAttribute('data-yav-magnetic-strength')||'0.32');var mx=((x-w/2)*str).toFixed(1),my=((y-h/2)*str).toFixed(1);el.style.transition='transform 80ms linear';el.style.transform='translate3d('+mx+'px,'+my+'px,0)';el.__yav_mag=1}else if(el.__yav_mag){el.__yav_mag=0;el.style.transition='transform 450ms cubic-bezier(0.175,0.885,0.32,1.275)';el.style.transform='translate3d(0,0,0)'}}});fl.forEach(function(f){f.x+=(px-f.x)*f.s;f.y+=(py-f.y)*f.s;f.el.style.transform='translate3d('+f.x.toFixed(1)+'px,'+f.y.toFixed(1)+'px,0)'});if(fl.length>0&&Math.hypot(px-fl[0].x,py-fl[0].y)>0.5)raf=requestAnimationFrame(tick)}function initFollowers(){fl=[];document.querySelectorAll('[yav\\:cursor="follower"],[data-yav-cursor="follower"]').forEach(function(el){fl.push({el:el,x:px,y:py,s:parseFloat(el.getAttribute('yav:cursor-smoothing')||'0.22')})})}window.addEventListener('pointermove',function(e){px=e.clientX;py=e.clientY;if(!raf)raf=requestAnimationFrame(tick)},{passive:true});window.addEventListener('yav:navigate',initFollowers);initFollowers()})();</script>"##;

/// Modular motion, harmonic spring physics, scroll-spy, count-up, bar animation & typewriter runtime.
/// Auto-injected by SSR tree-shaking when motion/scroll/counter/typewriter attributes are present.
pub const YAV_MOTION_RUNTIME_SCRIPT: &str = r##"<script id="yav-motion-runtime">(function(){if(window.__YAV_MOTION_INSTALLED__)return;window.__YAV_MOTION_INSTALLED__=true;var red=function(){return window.matchMedia&&window.matchMedia('(prefers-reduced-motion: reduce)').matches};function parseShorthand(s){if(!s)return null;if(s.charAt(0)==='{'){try{return JSON.parse(s)}catch(_){return null}}var o={};s.split(',').forEach(function(p){var kv=p.split(':');if(kv.length===2){var k=kv[0].trim(),v=parseFloat(kv[1].trim());if(!isNaN(v))o[k]=v}});return o}function parseSpring(el){var sp=el.getAttribute('yav:spring')||el.getAttribute('data-yav-spring');if(sp){var pts=sp.split(',').map(function(x){return parseFloat(x.trim())});return{is_spring:true,stiffness:pts[0]||260,damping:pts[1]||20,mass:pts[2]||1}}var tr=el.getAttribute('data-motion-transition');if(tr){try{return JSON.parse(tr)}catch(_){}}return{is_spring:true,stiffness:260,damping:20,mass:1,duration_ms:400}}function solveSpring(k,c,m){var pos=0,v=0,dt=1/60,frames=[0],maxSteps=180;for(var i=0;i<maxSteps;i++){var f=-k*(pos-1)-c*v;var a=f/(m||1);v+=a*dt;pos+=v*dt;frames.push(pos);if(i>6&&Math.abs(pos-1)<0.0015&&Math.abs(v)<0.0015)break}frames.push(1);return{frames:frames,duration:Math.min(frames.length*16.67,1800)}}function buildTransform(st,p){var t=[];var x=(st.x||0)*p,y=(st.y||0)*p;if(st.x!==undefined||st.y!==undefined)t.push('translate3d('+x.toFixed(2)+'px,'+y.toFixed(2)+'px,0)');if(st.scale!==undefined){var sc=1+(st.scale-1)*p;t.push('scale('+sc.toFixed(4)+')')}if(st.rotate!==undefined){var r=st.rotate*p;t.push('rotate('+r.toFixed(2)+'deg)')}return t.length?t.join(' '):'none'}function animateTo(el,target,trans){if(!target)return;var tr=trans||parseSpring(el);if(red()){el.style.transition='none';if(target.opacity!==undefined)el.style.opacity=target.opacity;el.style.transform=buildTransform(target,1);return}if(tr.is_spring&&el.animate){var sp=solveSpring(tr.stiffness||260,tr.damping||20,tr.mass||1);var kf=sp.frames.map(function(p){var frame={transform:buildTransform(target,p)};if(target.opacity!==undefined)frame.opacity=target.opacity;return frame});el.animate(kf,{duration:sp.duration,fill:'forwards',easing:'linear'});return}var dur=tr.duration_ms||400,ease=tr.ease||'cubic-bezier(0.16,1,0.3,1)';el.style.transition='all '+dur+'ms '+ease;if(target.opacity!==undefined)el.style.opacity=target.opacity;el.style.transform=buildTransform(target,1);if(target.blur!==undefined)el.style.filter='blur('+target.blur+'px)'}function runCount(el){var raw=el.getAttribute('yav:count-to')||el.getAttribute('data-count');if(!raw)return;var to=parseFloat(raw);if(isNaN(to))return;var decAttr=el.getAttribute('yav:count-decimals');var dec=decAttr!==null?parseInt(decAttr,10):(raw.split('.')[1]?raw.split('.')[1].length:0);var dur=red()?0:parseInt(el.getAttribute('yav:count-duration')||'1400',10);var pref=el.getAttribute('yav:count-prefix')||'',suff=el.getAttribute('yav:count-suffix')||'';var t0=performance.now();function step(now){var p=dur>0?Math.min((now-t0)/dur,1):1;var k=1-Math.pow(1-p,3);var val=to*k;el.textContent=pref+val.toLocaleString('en-US',{minimumFractionDigits:dec,maximumFractionDigits:dec})+suff;if(p<1)el.__yav_raf=requestAnimationFrame(step)}if(el.__yav_raf)cancelAnimationFrame(el.__yav_raf);el.__yav_raf=requestAnimationFrame(step)}function runBar(el){var w=el.getAttribute('yav:bar-to');if(!w)return;var dur=red()?0:parseInt(el.getAttribute('yav:bar-duration')||'1400',10);el.style.transition='none';el.style.width='0%';requestAnimationFrame(function(){requestAnimationFrame(function(){el.style.transition='width '+dur+'ms cubic-bezier(0.2,0.8,0.2,1)';el.style.width=w})})}function runTypewriter(el){var speed=parseInt(el.getAttribute('yav:typewriter')||el.getAttribute('data-yav-typewriter')||'14',10);var curChar=el.getAttribute('yav:typewriter-cursor')||'▋';if(!el.__yav_orig_html)el.__yav_orig_html=el.innerHTML;if(el.__yav_tw_tm)clearTimeout(el.__yav_tw_tm);if(red()){el.innerHTML=el.__yav_orig_html;return}var tmp=document.createElement('div');tmp.innerHTML=el.__yav_orig_html;var lines=Array.from(tmp.children);if(!lines.length)return;el.innerHTML='';el.__yav_typing=true;var cur=document.createElement('span');cur.className='code-cursor';cur.textContent=curChar;var li=0;function nextLine(){if(!el.__yav_typing)return;if(li>=lines.length){el.__yav_typing=false;if(el.lastElementChild)el.lastElementChild.appendChild(cur);return}var src=lines[li],row=document.createElement('div');el.appendChild(row);var ln=src.querySelector('.ln');if(ln)row.appendChild(ln.cloneNode(true));row.appendChild(cur);var chars=[];Array.from(src.childNodes).forEach(function(n){if(n.nodeType===1&&n.classList.contains('ln'))return;var tx=n.textContent||'',cl=n.nodeType===1?n.className:null;for(var i=0;i<tx.length;i++)chars.push({c:tx[i],cl:cl})});var ci=0,sp=null;function nextChar(){if(!el.__yav_typing)return;if(ci>=chars.length){li++;el.__yav_tw_tm=setTimeout(nextLine,38);return}var ch=chars[ci++];if(ch.cl){if(sp&&sp.className===ch.cl)sp.textContent+=ch.c;else{sp=document.createElement('span');sp.className=ch.cl;sp.textContent=ch.c;row.insertBefore(sp,cur)}}else{sp=null;row.insertBefore(document.createTextNode(ch.c),cur)}el.__yav_tw_tm=setTimeout(nextChar,ch.c===' '?5:speed)}nextChar()}nextLine()}function skipTypewriter(el){if(!el.__yav_orig_html)return;if(el.__yav_tw_tm)clearTimeout(el.__yav_tw_tm);el.__yav_typing=false;el.innerHTML=el.__yav_orig_html}function onScroll(){var y=window.scrollY||document.documentElement.scrollTop||0;var max=document.documentElement.scrollHeight-window.innerHeight;var p=max>0?Math.min(Math.max(y/max,0),1):0;document.querySelectorAll('[data-scroll-progress],[yav\\:scroll-progress],[data-yav-scroll-progress]').forEach(function(b){b.style.setProperty('--progress',p);b.style.transform='scaleX('+p+')'});document.querySelectorAll('[yav\\:scroll-class],[data-yav-scroll-class]').forEach(function(el){var spec=el.getAttribute('yav:scroll-class')||el.getAttribute('data-yav-scroll-class')||'is-scrolled@8';var pts=spec.split('@'),cls=pts[0]||'is-scrolled',th=parseFloat(pts[1]||'8');el.classList.toggle(cls,y>th)})}var io='IntersectionObserver' in window?new IntersectionObserver(function(es){es.forEach(function(e){if(!e.isIntersecting)return;var t=e.target;if(t.hasAttribute('yav:reveal')||t.hasAttribute('data-yav-reveal')||t.classList.contains('reveal')){var rc=t.getAttribute('yav:reveal')||t.getAttribute('data-yav-reveal')||'is-in';t.classList.add(rc,'is-in')}if(t.hasAttribute('yav:count-to')||t.hasAttribute('data-count'))runCount(t);if(t.hasAttribute('yav:bar-to'))runBar(t);if(t.hasAttribute('yav:typewriter')||t.hasAttribute('data-yav-typewriter'))runTypewriter(t);if(t.hasAttribute('data-motion')||t.hasAttribute('yav:animate')){var anim=parseShorthand(t.getAttribute('yav:animate')||t.getAttribute('data-motion-animate'));if(anim)animateTo(t,anim,parseSpring(t))}if(t.getAttribute('data-motion-once')!=='false')io.unobserve(t)})},{threshold:0.12}):null;function initMotion(){onScroll();document.querySelectorAll('.reveal,[yav\\:reveal],[data-yav-reveal],[yav\\:count-to],[data-count],[yav\\:bar-to],[yav\\:typewriter],[data-yav-typewriter],[data-motion-inview="true"]').forEach(function(el){if(io&&!el.__yav_io){el.__yav_io=1;io.observe(el)}});document.querySelectorAll('[data-motion-hover],[data-motion-tap],[yav\\:hover],[yav\\:tap]').forEach(function(el){if(el.__yav_gest)return;el.__yav_gest=1;var hov=parseShorthand(el.getAttribute('yav:hover')||el.getAttribute('data-motion-hover'));var tap=parseShorthand(el.getAttribute('yav:tap')||el.getAttribute('data-motion-tap'));var rest={x:0,y:0,scale:1,rotate:0,opacity:1};if(hov){el.addEventListener('mouseenter',function(){animateTo(el,hov,parseSpring(el))});el.addEventListener('mouseleave',function(){animateTo(el,rest,parseSpring(el))})}if(tap){el.addEventListener('mousedown',function(){animateTo(el,tap,{duration_ms:90})});el.addEventListener('mouseup',function(){animateTo(el,hov||rest,parseSpring(el))})}});document.querySelectorAll('[yav\\:typewriter-skip="click"]').forEach(function(el){if(el.__yav_tw_sk)return;el.__yav_tw_sk=1;el.addEventListener('click',function(){skipTypewriter(el)})});document.querySelectorAll('.toc,[yav\\:scroll-spy],[data-yav-scroll-spy]').forEach(function(toc){if(toc.__yav_spy||!('IntersectionObserver' in window))return;toc.__yav_spy=1;var actCls=toc.getAttribute('yav:scroll-spy')||toc.getAttribute('data-yav-scroll-spy')||'is-active';var links={};toc.querySelectorAll('a[href^="#"]').forEach(function(a){var h=a.getAttribute('href');if(h&&h.length>1)links[h.slice(1)]=a});var sio=new IntersectionObserver(function(es){es.forEach(function(e){if(e.isIntersecting){Object.keys(links).forEach(function(k){links[k].classList.toggle(actCls,k===e.target.id)})}})},{rootMargin:'-20% 0px -60% 0px'});Object.keys(links).forEach(function(id){var s=document.getElementById(id);if(s)sio.observe(s)})})}document.addEventListener('click',function(e){var rp=e.target.closest('[yav\\:replay],[data-yav-replay]');if(rp){var sel=rp.getAttribute('yav:replay')||rp.getAttribute('data-yav-replay');var box=sel==='self'?rp:document.querySelector(sel);if(box){box.querySelectorAll('[yav\\:count-to],[data-count]').forEach(runCount);box.querySelectorAll('[yav\\:bar-to]').forEach(runBar);box.querySelectorAll('[yav\\:typewriter],[data-yav-typewriter]').forEach(runTypewriter)}}});window.addEventListener('yav:tab-change',function(e){var p=e.detail&&e.detail.panel;if(p){if(p.hasAttribute('yav:typewriter')||p.hasAttribute('data-yav-typewriter'))runTypewriter(p);p.querySelectorAll('[yav\\:typewriter],[data-yav-typewriter]').forEach(runTypewriter)}});window.addEventListener('scroll',onScroll,{passive:true});window.addEventListener('yav:navigate',initMotion);if(document.readyState==='loading')document.addEventListener('DOMContentLoaded',initMotion);else initMotion()})();</script>"##;

/// Modular client-side reactive signals & complex widget runtime (`yav:state`, `yav:bind`, `yav:text`, `yav:show`, `yav:on-click`, `yav:style-width`, `KanbanBoard`, `FlowCanvas`, `VirtualList`, `RichTextEditor`, `AiChat`).
/// CSP-safe expression evaluator — does NOT use `new Function()` or `eval()`,
/// so it works under strict `Content-Security-Policy` without `unsafe-eval`.
/// Auto-injected by SSR tree-shaking only when reactive state or complex widget attributes are present.
pub const YAV_SIGNALS_RUNTIME_SCRIPT: &str = r##"<script id="yav-signals-runtime">(function(){if(window.__YAV_SIGNALS__)return;window.__YAV_SIGNALS__=1;function resolveToken(tok,st){var t=tok.trim();if(t==='true')return true;if(t==='false')return false;if(t==='null')return null;if((t.charAt(0)==='"'&&t.charAt(t.length-1)==='"')||(t.charAt(0)==="'"&&t.charAt(t.length-1)==="'"))return t.slice(1,-1);var n=Number(t);if(!isNaN(n)&&t!=='')return n;if(t.charAt(0)==='!')return !resolveToken(t.slice(1),st);return st[t]}function evalInScope(expr,state){if(!expr)return '';var s=expr.trim();if(Object.prototype.hasOwnProperty.call(state,s))return state[s];if(s.charAt(0)==='!'&&Object.prototype.hasOwnProperty.call(state,s.slice(1).trim()))return !state[s.slice(1).trim()];var qIdx=s.indexOf('?'),cIdx=s.indexOf(':');if(qIdx!==-1&&cIdx>qIdx){var cond=evalInScope(s.slice(0,qIdx),state);return cond?evalInScope(s.slice(qIdx+1,cIdx),state):evalInScope(s.slice(cIdx+1),state)}var ops=['===','!==','>=','<=','==','!=','>','<','+','-','*','/','%'];for(var i=0;i<ops.length;i++){var op=ops[i],idx=s.indexOf(op);if(idx>0){var l=resolveToken(s.slice(0,idx),state),r=resolveToken(s.slice(idx+op.length),state);if(op==='+')return l+r;if(op==='-')return Number(l)-Number(r);if(op==='*')return Number(l)*Number(r);if(op==='/')return Number(r)!==0?Number(l)/Number(r):0;if(op==='%')return Number(l)%Number(r);if(op==='==='||op==='==')return l===r;if(op==='!=='||op==='!=')return l!==r;if(op==='>=')return l>=r;if(op==='<=')return l<=r;if(op==='>')return l>r;if(op==='<')return l<r}}try{var keys=Object.keys(state),vals=keys.map(function(k){return state[k]});return resolveToken(s,state)}catch(_){return resolveToken(s,state)}}function mutateInScope(stmt,state){if(!stmt)return;stmt.split(';').forEach(function(part){var p=part.trim();if(!p)return;if(p.slice(-2)==='++'){var k=p.slice(0,-2).trim();state[k]=(Number(state[k])||0)+1;return}if(p.slice(-2)==='--'){var k2=p.slice(0,-2).trim();state[k2]=(Number(state[k2])||0)-1;return}var pe=p.indexOf('+=');if(pe>0){var kp=p.slice(0,pe).trim();state[kp]=(Number(state[kp])||0)+Number(evalInScope(p.slice(pe+2),state));return}var me=p.indexOf('-=');if(me>0){var km=p.slice(0,me).trim();state[km]=(Number(state[km])||0)-Number(evalInScope(p.slice(me+2),state));return}var eq=p.indexOf('=');if(eq>0&&p.charAt(eq-1)!=='!'&&p.charAt(eq-1)!=='='&&p.charAt(eq+1)!=='='){var ke=p.slice(0,eq).trim();state[ke]=evalInScope(p.slice(eq+1),state);return}try{void 0}catch(_){}})}function renderScope(root){var st=root.__yav_state;if(!st)return;root.querySelectorAll('[yav\\:text],[data-yav-text]').forEach(function(el){var ex=el.getAttribute('yav:text')||el.getAttribute('data-yav-text');var v=evalInScope(ex,st);if(v!==undefined&&v!==null)el.textContent=String(v)});root.querySelectorAll('[yav\\:show],[data-yav-show-if]').forEach(function(el){var ex=el.getAttribute('yav:show')||el.getAttribute('data-yav-show-if');if(ex&&ex.charAt(0)!=='#'){var v=Boolean(evalInScope(ex,st));el.classList.toggle('hidden',!v);if(v)el.removeAttribute('hidden');else el.setAttribute('hidden','')}});root.querySelectorAll('[yav\\:style-width],[data-yav-style-width]').forEach(function(el){var ex=el.getAttribute('yav:style-width')||el.getAttribute('data-yav-style-width');var v=evalInScope(ex,st);if(v!==undefined)el.style.width=String(v)});root.querySelectorAll('[yav\\:bind],[data-yav-bind]').forEach(function(el){var k=el.getAttribute('yav:bind')||el.getAttribute('data-yav-bind');if(st[k]!==undefined&&document.activeElement!==el)el.value=st[k]})}function lexoMid(a,b){if(!a&&!b)return 'U';if(!a){var c=b.charCodeAt(0);return (c>48?String.fromCharCode(Math.max(48,Math.floor((48+c)/2))):'0')+b}if(!b)return a+'U';if(a===b)return a+'U';var m=Math.min(a.length,b.length),r='',i=0;for(;i<m;i++){var ca=a.charCodeAt(i),cb=b.charCodeAt(i);if(ca===cb){r+=a.charAt(i);continue}if(cb-ca>1){return r+String.fromCharCode(Math.floor((ca+cb)/2))}else{return r+a.charAt(i)+'U'}}return a+'U'}var dragCard=null;document.addEventListener('dragstart',function(e){var c=e.target.closest('.yav-kanban-card[draggable="true"]');if(c){dragCard=c;c.classList.add('opacity-50')}});document.addEventListener('dragend',function(){if(dragCard){dragCard.classList.remove('opacity-50');dragCard=null}});document.addEventListener('dragover',function(e){if(dragCard&&e.target.closest('.yav-kanban-column-list'))e.preventDefault()});document.addEventListener('drop',function(e){var col=e.target.closest('.yav-kanban-column-list');if(!col||!dragCard)return;e.preventDefault();var after=e.target.closest('.yav-kanban-card');if(after&&after!==dragCard)col.insertBefore(dragCard,after);else col.appendChild(dragCard);var prev=dragCard.previousElementSibling,next=dragCard.nextElementSibling;var rk=lexoMid(prev?prev.getAttribute('data-rank'):null,next?next.getAttribute('data-rank'):null);dragCard.setAttribute('data-rank',rk);var board=col.closest('.yav-kanban');if(board){var u=board.getAttribute('yav:drop-url');if(u)fetch(u,{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({cardId:dragCard.getAttribute('data-card-id'),columnId:col.getAttribute('data-column-id'),rank:rk})}).catch(function(){})}});var flowNode=null,fx=0,fy=0;document.addEventListener('pointerdown',function(e){var n=e.target.closest('.yav-flow-node');if(n){flowNode=n;var m=/translate\(([-\d.]+)px,\s*([-\d.]+)px\)/.exec(n.getAttribute('style')||'');fx=e.clientX-(m?parseFloat(m[1]):0);fy=e.clientY-(m?parseFloat(m[2]):0);n.setPointerCapture(e.pointerId)}});document.addEventListener('pointermove',function(e){if(!flowNode)return;var nx=Math.max(0,e.clientX-fx),ny=Math.max(0,e.clientY-fy);flowNode.style.transform='translate('+nx.toFixed(1)+'px,'+ny.toFixed(1)+'px)';flowNode.setAttribute('data-x',nx.toFixed(1));flowNode.setAttribute('data-y',ny.toFixed(1));var cv=flowNode.closest('.yav-flow-canvas');if(cv){cv.querySelectorAll('path[data-from][data-to]').forEach(function(pt){var fId=pt.getAttribute('data-from'),tId=pt.getAttribute('data-to');var sEl=cv.querySelector('.yav-flow-node[data-node-id="'+fId+'"]'),dEl=cv.querySelector('.yav-flow-node[data-node-id="'+tId+'"]');if(sEl&&dEl){var x1=parseFloat(sEl.getAttribute('data-x')||'0')+140,y1=parseFloat(sEl.getAttribute('data-y')||'0')+28;var x2=parseFloat(dEl.getAttribute('data-x')||'0'),y2=parseFloat(dEl.getAttribute('data-y')||'0')+28;var dx=Math.max(Math.abs(x2-x1)*0.5,40),cx1=x1+dx,cx2=x2-dx;pt.setAttribute('d','M '+x1.toFixed(1)+' '+y1.toFixed(1)+' C '+cx1.toFixed(1)+' '+y1.toFixed(1)+', '+cx2.toFixed(1)+' '+y2.toFixed(1)+', '+x2.toFixed(1)+' '+y2.toFixed(1));var lb=cv.querySelector('text[data-edge-label-for="'+fId+'->'+tId+'"]');if(lb){lb.setAttribute('x',((x1+x2)/2).toFixed(1));lb.setAttribute('y',((y1+y2)/2-6).toFixed(1))}}})}});document.addEventListener('pointerup',function(){flowNode=null});function initSignals(){document.querySelectorAll('[yav\\:state],[data-yav-state]').forEach(function(root){if(root.__yav_state)return;var raw=root.getAttribute('yav:state')||root.getAttribute('data-yav-state')||'{}';var st={};if(raw.trim().charAt(0)==='{'){try{st=JSON.parse(raw)}catch(_){}}else{raw.split(',').forEach(function(p){var kv=p.split(':');if(kv.length===2){var k=kv[0].trim(),v=kv[1].trim();var n=Number(v);st[k]=!isNaN(n)&&v!==''?n:(v==='true'?true:(v==='false'?false:v.replace(/^['"]|['"]$/g,'')))}})}root.__yav_state=st;renderScope(root)});document.querySelectorAll('.yav-virtual-list[yav\\:virtual-fetch]').forEach(function(vl){if(vl.__yav_vl)return;vl.__yav_vl=1;vl.addEventListener('scroll',function(){var rh=parseInt(vl.getAttribute('data-row-height')||'40',10);var start=Math.floor(vl.scrollTop/rh);var u=vl.getAttribute('yav:virtual-fetch');var win=vl.querySelector('.yav-virtual-window');if(win)win.style.transform='translateY('+(start*rh)+'px)';if(u&&win){fetch(u+(u.indexOf('?')===-1?'?':'&')+'start='+start).then(function(r){return r.text()}).then(function(h){win.innerHTML=h}).catch(function(){})}},{passive:true})})}document.addEventListener('input',function(e){var b=e.target.closest('[yav\\:bind],[data-yav-bind]');if(b){var root=b.closest('[yav\\:state],[data-yav-state]');if(root&&root.__yav_state){var k=b.getAttribute('yav:bind')||b.getAttribute('data-yav-bind');var n=Number(b.value);root.__yav_state[k]=!isNaN(n)&&b.value!==''?n:b.value;renderScope(root)}}var rte=e.target.closest('.yav-rte-surface');if(rte){var box=rte.closest('.yav-rich-editor');var inp=box?box.querySelector('input[type="hidden"]'):null;if(inp)inp.value=rte.innerHTML}});document.addEventListener('click',function(e){var act=e.target.closest('[yav\\:on-click],[data-yav-on-click]');if(act){var root=act.closest('[yav\\:state],[data-yav-state]');if(root&&root.__yav_state){var stmt=act.getAttribute('yav:on-click')||act.getAttribute('data-yav-on-click');mutateInScope(stmt,root.__yav_state);renderScope(root)}}var cmdBtn=e.target.closest('[data-rte-cmd]');if(cmdBtn){var cmd=cmdBtn.getAttribute('data-rte-cmd');var editor=cmdBtn.closest('.yav-rich-editor');var surf=editor?editor.querySelector('.yav-rte-surface'):null;if(surf){surf.focus();if(cmd==='bold'||cmd==='italic')document.execCommand(cmd,false,null);else if(cmd==='h1'||cmd==='h2')document.execCommand('formatBlock',false,cmd.toUpperCase());else if(cmd==='code')document.execCommand('formatBlock',false,'PRE');var hi=editor.querySelector('input[type="hidden"]');if(hi)hi.value=surf.innerHTML}}});document.addEventListener('submit',function(e){var af=e.target.closest('form[yav\\:ai-stream]');if(af){e.preventDefault();var u=af.getAttribute('yav:ai-stream');var inp=af.querySelector('input[name="prompt"]');var prompt=inp?inp.value.trim():'';if(!prompt||!u)return;if(inp)inp.value='';var chat=af.closest('.yav-ai-chat');var log=chat?chat.querySelector('[role="log"]'):null;if(log){var ub=document.createElement('div');ub.className='ml-auto max-w-[80%] rounded-2xl bg-indigo-600 text-white px-4 py-2 text-sm';ub.textContent=prompt;log.appendChild(ub);var ab=document.createElement('div');ab.className='mr-auto max-w-[80%] rounded-2xl bg-gray-100 dark:bg-gray-800 text-gray-900 dark:text-gray-100 px-4 py-2 text-sm';log.appendChild(ab);fetch(u,{method:'POST',headers:{'Content-Type':'application/x-www-form-urlencoded'},body:'prompt='+encodeURIComponent(prompt)}).then(function(r){return r.text()}).then(function(txt){ab.textContent=txt}).catch(function(){})}}});window.addEventListener('yav:navigate',initSignals);if(document.readyState==='loading')document.addEventListener('DOMContentLoaded',initSignals);else initSignals()})();</script>"##;

/// Inspect rendered HTML to determine which optional micro-runtime modules should be tree-shaken in.
fn append_tree_shaken_runtimes(html_context: &str, out: &mut String) {
    if !html_context.contains("id=\"yav-micro-actions\"") {
        out.push_str(YAV_MICRO_ACTIONS_SCRIPT);
    }

    // Inject UI component actions (dismiss, theme toggle, copy code, seek video)
    let needs_ui_actions = html_context.contains("data-dismiss")
        || html_context.contains("data-theme-toggle")
        || html_context.contains("data-copy-code")
        || html_context.contains("data-seek-seconds");
    if needs_ui_actions && !html_context.contains("id=\"yav-ui-actions\"") {
        out.push_str(YAV_UI_ACTIONS_SCRIPT);
    }

    let needs_cursor = html_context.contains("yav:cursor")
        || html_context.contains("data-yav-cursor")
        || html_context.contains("yav:tilt-max")
        || html_context.contains("yav:magnetic-strength");
    if needs_cursor && !html_context.contains("id=\"yav-cursor-runtime\"") {
        out.push_str(YAV_CURSOR_RUNTIME_SCRIPT);
    }

    let needs_motion = html_context.contains("data-motion")
        || html_context.contains("yav:animate")
        || html_context.contains("yav:hover")
        || html_context.contains("yav:tap")
        || html_context.contains("yav:spring")
        || html_context.contains("yav:reveal")
        || html_context.contains("data-yav-reveal")
        || html_context.contains("yav:scroll-class")
        || html_context.contains("data-yav-scroll-class")
        || html_context.contains("yav:scroll-progress")
        || html_context.contains("data-scroll-progress")
        || html_context.contains("yav:scroll-spy")
        || html_context.contains("data-yav-scroll-spy")
        || html_context.contains("yav:count-to")
        || html_context.contains("data-count=")
        || html_context.contains("yav:bar-to")
        || html_context.contains("yav:typewriter")
        || html_context.contains("data-yav-typewriter")
        || html_context.contains("yav:replay")
        || html_context.contains("data-yav-replay")
        || html_context.contains("class=\"reveal")
        || html_context.contains("class=\"toc");
    if needs_motion
        && !html_context.contains("id=\"yav-motion-runtime\"")
        && !html_context.contains("__YAV_MOTION_INSTALLED__")
    {
        out.push_str(YAV_MOTION_RUNTIME_SCRIPT);
    }

    let needs_signals = html_context.contains("yav:state")
        || html_context.contains("data-yav-state")
        || html_context.contains("yav:bind")
        || html_context.contains("data-yav-bind")
        || html_context.contains("yav:on-click")
        || html_context.contains("data-yav-on-click")
        || html_context.contains("yav:text")
        || html_context.contains("data-yav-text")
        || html_context.contains("yav:style-width")
        || html_context.contains("data-yav-style-width")
        || html_context.contains("yav:show-if")
        || html_context.contains("data-yav-show-if")
        || html_context.contains("yav-kanban")
        || html_context.contains("yav-flow-canvas")
        || html_context.contains("yav-virtual-list")
        || html_context.contains("yav-rich-editor")
        || html_context.contains("yav:ai-stream");
    if needs_signals && !html_context.contains("id=\"yav-signals-runtime\"") {
        out.push_str(YAV_SIGNALS_RUNTIME_SCRIPT);
    }

    let needs_local_first = html_context.contains("yav:local-store")
        || html_context.contains("yav:offline-sync")
        || html_context.contains("yav:offline-queue");
    if needs_local_first && !html_context.contains("yav_local_first_db") {
        out.push_str(crate::local_first::YAV_LOCAL_FIRST_SCRIPT);
    }
}

fn is_head_portal_element(el: &crate::node::Element) -> bool {
    el.tag == "template" && el.attributes.iter().any(|a| a.name == "data-yav-head")
}

fn collect_head_elements<'a>(
    node: &'a Node,
    head_children: &mut Vec<&'a Node>,
    last_title: &mut Option<String>,
) {
    match node {
        Node::Element(el) => {
            if is_head_portal_element(el) {
                for child in &el.children {
                    if let Node::Element(child_el) = child {
                        if child_el.tag.eq_ignore_ascii_case("title") {
                            let mut t_buf = String::new();
                            for tc in &child_el.children {
                                render_node_depth(tc, &mut t_buf, 0);
                            }
                            *last_title = Some(t_buf);
                            continue;
                        }
                    }
                    head_children.push(child);
                }
            } else {
                for child in &el.children {
                    collect_head_elements(child, head_children, last_title);
                }
            }
        }
        Node::Fragment(children) => {
            for child in children {
                collect_head_elements(child, head_children, last_title);
            }
        }
        _ => {}
    }
}

fn render_node_skipping_heads(node: &Node, buf: &mut String, depth: usize) {
    if depth > MAX_RENDER_DEPTH {
        buf.push_str("<!-- yav: render depth limit exceeded -->");
        return;
    }
    match node {
        Node::Element(el) => {
            if is_head_portal_element(el) {
                return;
            }
            let safe_tag = if is_valid_tag_name(&el.tag) {
                el.tag.as_ref()
            } else {
                "div"
            };
            buf.push('<');
            buf.push_str(safe_tag);
            for attr in &el.attributes {
                if !is_valid_attribute_name(&attr.name) {
                    continue;
                }
                match &attr.value {
                    AttributeValue::String(val) => {
                        render_dynamic_string_attribute(&attr.name, val.as_str(), buf);
                    }
                    AttributeValue::Static(val) => {
                        render_dynamic_string_attribute(&attr.name, val, buf);
                    }
                    AttributeValue::Bool(true) | AttributeValue::None => {
                        buf.push(' ');
                        buf.push_str(&attr.name);
                    }
                    AttributeValue::Bool(false) => {}
                }
            }
            if el.self_closing && crate::node::is_void_element(safe_tag) {
                buf.push_str(" />");
                return;
            }
            buf.push('>');
            let is_script = safe_tag.eq_ignore_ascii_case("script");
            let is_style = safe_tag.eq_ignore_ascii_case("style");
            for child in &el.children {
                if is_script {
                    render_rawtext_child(child, buf, "</script", "<\\/script", depth + 1);
                } else if is_style {
                    render_rawtext_child(child, buf, "</style", "<\\/style", depth + 1);
                } else {
                    render_node_skipping_heads(child, buf, depth + 1);
                }
            }
            buf.push_str("</");
            buf.push_str(safe_tag);
            buf.push('>');
        }
        Node::Text(text) => buf.push_str(&escape_html(text)),
        Node::Raw(html) => buf.push_str(html),
        Node::Static(s) => buf.push_str(s),
        Node::Fragment(children) => {
            for child in children {
                render_node_skipping_heads(child, buf, depth + 1);
            }
        }
        Node::Empty => {}
    }
}

/// Zero-clone, single-allocation HTML5 document renderer directly from a [`Node`] tree.
///
/// Traverses `body` by reference to hoist `<Head>` elements into `<head>` without cloning
/// the Virtual DOM or allocating intermediate body buffers.
pub fn render_node_document_with_lang(
    title: &str,
    head_extra: &str,
    body: &Node,
    body_end: &str,
    lang: &str,
) -> String {
    let mut head_nodes = Vec::new();
    let mut extracted_title = None;
    collect_head_elements(body, &mut head_nodes, &mut extracted_title);

    let est_body = estimate_node_size(body).min(1024 * 1024);
    let mut out = String::with_capacity(est_body + head_extra.len() + body_end.len() + 2048);

    out.push_str("<!DOCTYPE html>\n<html lang=\"");
    out.push_str(&escape_attribute(lang));
    out.push_str("\">\n<head>\n<meta charset=\"utf-8\" />\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\" />\n<title>");
    match extracted_title {
        Some(ref t) => out.push_str(t),
        None => out.push_str(&escape_html(title)),
    }
    out.push_str("</title>\n");
    out.push_str(head_extra);
    for hn in head_nodes {
        render_node_depth(hn, &mut out, 0);
    }
    out.push_str("</head>\n<body>\n");
    render_node_skipping_heads(body, &mut out, 0);
    out.push('\n');
    out.push_str(body_end);
    let mut runtime_tail = String::new();
    append_tree_shaken_runtimes(&out, &mut runtime_tail);
    out.push_str(&runtime_tail);
    out.push_str("</body>\n</html>");

    // Also handle any raw `<template data-yav-head>` that might have been injected via Node::Raw
    if out.contains("<template data-yav-head") {
        let (cleaned, raw_title, raw_heads) = extract_head_templates(&out);
        let mut final_doc = cleaned;
        if let Some(rt) = raw_title {
            if let (Some(ts), Some(te)) = (final_doc.find("<title>"), final_doc.find("</title>")) {
                if ts + 7 <= te {
                    final_doc.replace_range(ts + 7..te, &rt);
                }
            }
        }
        if !raw_heads.is_empty() {
            if let Some(he) = final_doc.find("</head>") {
                final_doc.insert_str(he, &raw_heads);
            }
        }
        return final_doc;
    }

    out
}

/// Extract hoisted `<Head>` tags (`<template data-yav-head...>...</template>`) from rendered body HTML.
/// Returns `(clean_body, extracted_title, extracted_head_tags)`.
pub fn extract_head_templates(body: &str) -> (String, Option<String>, String) {
    let tag_open = "<template data-yav-head";
    if !body.contains(tag_open) {
        return (body.to_string(), None, String::new());
    }

    let mut clean_body = String::with_capacity(body.len());
    let mut last_idx = 0;
    let mut final_title = None;
    let mut head_tags = String::new();

    while let Some(start) = body[last_idx..].find(tag_open) {
        let abs_start = last_idx + start;
        clean_body.push_str(&body[last_idx..abs_start]);

        if let Some(open_tag_end) = body[abs_start..].find('>') {
            let content_start = abs_start + open_tag_end + 1;
            if let Some(close_tag_start) = body[content_start..].find("</template>") {
                let abs_close = content_start + close_tag_start;
                let inner = &body[content_start..abs_close];

                let mut rem = inner;
                while let Some(t_start) = rem.find("<title>") {
                    if let Some(t_end) = rem[t_start..].find("</title>") {
                        let title_content = &rem[t_start + 7..t_start + t_end];
                        final_title = Some(title_content.to_string());
                        head_tags.push_str(&rem[..t_start]);
                        rem = &rem[t_start + t_end + 8..];
                    } else {
                        break;
                    }
                }
                head_tags.push_str(rem);

                last_idx = abs_close + "</template>".len();
            } else {
                last_idx = abs_start + tag_open.len();
            }
        } else {
            last_idx = abs_start + tag_open.len();
        }
    }
    clean_body.push_str(&body[last_idx..]);

    (clean_body, final_title, head_tags)
}

pub fn document_shell(title: &str, head_extra: &str, body: &str, body_end: &str) -> String {
    document_shell_with_lang(title, head_extra, body, body_end, "en")
}

/// Wrap rendered body content in a full HTML5 document shell with custom `lang` attribute.
///
/// Automatically hoists any `<Head>` blocks from the body into `<head>`, updating `<title>`
/// and injecting `<meta>`, `<link>`, and other tags. Injects the lightweight Yav micro-actions runtime.
///
/// # Arguments
/// * `title` — the default `<title>` content
/// * `head_extra` — additional HTML to inject into `<head>` (CSS links, meta tags, etc.)
/// * `body` — the body content (already rendered HTML)
/// * `body_end` — scripts or other content to inject before `</body>`
/// * `lang` — the language code (e.g. `"en"`, `"fr"`, `"es"`)
pub fn document_shell_with_lang(
    title: &str,
    head_extra: &str,
    body: &str,
    body_end: &str,
    lang: &str,
) -> String {
    let (clean_body, extracted_title, extracted_head_tags) = extract_head_templates(body);

    let doc_title = match extracted_title {
        Some(t) => t,
        None => escape_html(title).into_owned(),
    };

    let effective_head = if extracted_head_tags.is_empty() {
        head_extra.to_string()
    } else {
        format!("{head_extra}{extracted_head_tags}")
    };

    let mut effective_body_end = body_end.to_string();
    let combined_check = format!("{clean_body}{body_end}");
    append_tree_shaken_runtimes(&combined_check, &mut effective_body_end);

    format!(
        r#"<!DOCTYPE html>
<html lang="{lang}">
<head>
<meta charset="utf-8" />
<meta name="viewport" content="width=device-width, initial-scale=1" />
<title>{doc_title}</title>
{effective_head}</head>
<body>
{clean_body}
{effective_body_end}</body>
</html>"#,
        lang = escape_attribute(lang),
        doc_title = doc_title,
        effective_head = effective_head,
        clean_body = clean_body,
        effective_body_end = effective_body_end,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::{Element, Node};

    #[test]
    fn head_template_hoisting_test() {
        let body = r#"<main><h1>Hello</h1><template data-yav-head><title>Custom Page Title</title><meta name="description" content="A great page" /><link rel="canonical" href="https://example.com" /></template><p>Body text</p></main>"#;
        let html = document_shell_with_lang("Default Title", "", body, "", "en");
        assert!(html.contains("<title>Custom Page Title</title>"));
        assert!(html.contains(r#"<meta name="description" content="A great page" />"#));
        assert!(html.contains(r#"<link rel="canonical" href="https://example.com" />"#));
        assert!(!html.contains("data-yav-head"));
        assert!(html.contains("<main><h1>Hello</h1><p>Body text</p></main>"));
    }

    #[test]
    fn render_text() {
        let node = Node::text("Hello <world>");
        assert_eq!(render_to_string(&node), "Hello &lt;world&gt;");
    }

    #[test]
    fn render_raw() {
        let node = Node::raw("<b>bold</b>");
        assert_eq!(render_to_string(&node), "<b>bold</b>");
    }

    #[test]
    fn render_static() {
        let node = Node::static_str(r#"<div class="hero">Static Content</div>"#);
        assert_eq!(
            render_to_string(&node),
            r#"<div class="hero">Static Content</div>"#
        );
    }

    #[test]
    fn render_empty() {
        let node = Node::empty();
        assert_eq!(render_to_string(&node), "");
    }

    #[test]
    fn render_simple_element() {
        let node = Node::Element(Element::new("div"));
        assert_eq!(render_to_string(&node), "<div></div>");
    }

    #[test]
    fn render_element_with_text() {
        let node = Node::Element(Element::new("p").child(Node::text("Hello")));
        assert_eq!(render_to_string(&node), "<p>Hello</p>");
    }

    #[test]
    fn render_element_with_attributes() {
        let node = Node::Element(
            Element::new("a")
                .attr("href", "/about")
                .attr("class", "link"),
        );
        assert_eq!(
            render_to_string(&node),
            r#"<a href="/about" class="link"></a>"#
        );
    }

    #[test]
    fn render_void_element() {
        let node = Node::Element(Element::new("br"));
        assert_eq!(render_to_string(&node), "<br />");
    }

    #[test]
    fn render_non_void_element_marked_self_closing() {
        let mut button = Element::new("button");
        button.self_closing = true;
        assert_eq!(
            render_to_string(&Node::Element(button)),
            "<button></button>"
        );

        let mut div = Element::new("div");
        div.self_closing = true;
        assert_eq!(render_to_string(&Node::Element(div)), "<div></div>");
    }

    #[test]
    fn render_document_shell_with_lang_test() {
        let html = document_shell_with_lang("Titre", "", "<p>Bonjour</p>", "", "fr");
        assert!(html.contains(r#"<html lang="fr">"#));
        assert!(html.contains("<title>Titre</title>"));
    }

    #[test]
    fn render_input_with_attrs() {
        let node = Node::Element(
            Element::new("input")
                .attr("type", "text")
                .attr("name", "query")
                .bool_attr("required", true),
        );
        assert_eq!(
            render_to_string(&node),
            r#"<input type="text" name="query" required />"#
        );
    }

    #[test]
    fn render_bool_attr_false_omitted() {
        let node = Node::Element(Element::new("input").bool_attr("disabled", false));
        assert_eq!(render_to_string(&node), "<input />");
    }

    #[test]
    fn render_nested_elements() {
        let node = Node::Element(
            Element::new("div")
                .attr("class", "outer")
                .child(Node::Element(
                    Element::new("span").child(Node::text("inner")),
                )),
        );
        assert_eq!(
            render_to_string(&node),
            r#"<div class="outer"><span>inner</span></div>"#
        );
    }

    #[test]
    fn render_fragment() {
        let node = Node::Fragment(vec![Node::text("one"), Node::text(" "), Node::text("two")]);
        assert_eq!(render_to_string(&node), "one two");
    }

    #[test]
    fn render_attribute_escaping() {
        let node = Node::Element(Element::new("div").attr("data-value", r#"say "hello" & 'bye'"#));
        assert_eq!(
            render_to_string(&node),
            r#"<div data-value="say &quot;hello&quot; &amp; &#x27;bye&#x27;"></div>"#
        );
    }

    #[test]
    fn render_xss_prevention() {
        let node =
            Node::Element(Element::new("p").child(Node::text("<script>alert('xss')</script>")));
        let html = render_to_string(&node);
        assert!(!html.contains("<script>"));
        assert!(html.contains("&lt;script&gt;"));
    }

    #[test]
    fn render_document_shell() {
        let html = document_shell(
            "My App",
            r#"<link rel="stylesheet" href="/style.css" />"#,
            "<h1>Hello</h1>",
            "",
        );
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("<title>My App</title>"));
        assert!(html.contains("style.css"));
        assert!(html.contains("<h1>Hello</h1>"));
    }

    #[test]
    fn render_complex_tree() {
        let node = Node::Element(
            Element::new("main")
                .attr("class", "container")
                .child(Node::Element(
                    Element::new("nav")
                        .child(Node::Element(
                            Element::new("a")
                                .attr("href", "/")
                                .child(Node::text("Home")),
                        ))
                        .child(Node::Element(
                            Element::new("a")
                                .attr("href", "/about")
                                .child(Node::text("About")),
                        )),
                ))
                .child(Node::Element(
                    Element::new("section")
                        .child(Node::Element(
                            Element::new("h1").child(Node::text("Welcome")),
                        ))
                        .child(Node::Element(
                            Element::new("p").child(Node::text("Build with Rust.")),
                        )),
                )),
        );

        let html = render_to_string(&node);
        assert!(html.starts_with(r#"<main class="container">"#));
        assert!(html.contains(r#"<a href="/">Home</a>"#));
        assert!(html.contains("<h1>Welcome</h1>"));
        assert!(html.ends_with("</main>"));
    }

    #[test]
    fn test_render_src_xss_protection() {
        let node = Node::Element(Element::new("iframe").attr("src", "javascript:alert(1)"));
        let html = render_to_string(&node);
        assert_eq!(html, r#"<iframe></iframe>"#);

        let safe_img = Node::Element(
            Element::new("img")
                .attr("src", "https://example.com/logo.png")
                .attr("alt", "Logo"),
        );
        let safe_html = render_to_string(&safe_img);
        assert_eq!(
            safe_html,
            r#"<img src="https://example.com/logo.png" alt="Logo" />"#
        );
    }

    #[test]
    fn test_render_multimedia_and_svg_url_protection() {
        // xlink:href on SVG <a>
        let svg_link = Node::Element(
            Element::new("a")
                .attr("xlink:href", "javascript:alert('svg')")
                .child(Node::text("SVG link")),
        );
        let html1 = render_to_string(&svg_link);
        assert_eq!(html1, r#"<a>SVG link</a>"#);

        // video poster
        let video =
            Node::Element(Element::new("video").attr("poster", "javascript:alert('poster')"));
        let html2 = render_to_string(&video);
        assert_eq!(html2, r#"<video></video>"#);

        // object data
        let obj = Node::Element(Element::new("object").attr("data", "javascript:alert('data')"));
        let html3 = render_to_string(&obj);
        assert_eq!(html3, r#"<object></object>"#);

        // Invalid attribute names with injection attempts must be dropped
        let injected_attr =
            Node::Element(Element::new("div").attr("foo\" onfocus=\"alert(1)", "bar"));
        let html4 = render_to_string(&injected_attr);
        assert_eq!(html4, "<div></div>");

        // Invalid tag names with injection attempts must fall back to <div>
        let injected_tag = Node::Element(
            Element::new("\"><script>alert(1)</script><div foo=\"").child(Node::text("test")),
        );
        let html5 = render_to_string(&injected_tag);
        assert_eq!(html5, "<div>test</div>");

        // Case-insensitive URL attributes (HREF, Src, formAction) must also be filtered
        let upper_href = Node::Element(
            Element::new("a")
                .attr("HREF", "javascript:alert('upper')")
                .child(Node::text("link")),
        );
        assert_eq!(render_to_string(&upper_href), "<a>link</a>");

        let mixed_formaction = Node::Element(
            Element::new("button")
                .attr("formAction", "javascript:alert('fa')")
                .child(Node::text("btn")),
        );
        assert_eq!(render_to_string(&mixed_formaction), "<button>btn</button>");
    }

    #[test]
    fn test_tree_shaken_signals_and_complex_widgets_runtime() {
        let kanban_body = Node::Element(Element::new("div").attr("class", "yav-kanban"));
        let doc = document_shell("Kanban", "", &render_to_string(&kanban_body), "");
        assert!(doc.contains("id=\"yav-signals-runtime\""));
        assert!(doc.contains("yav-kanban-card"));
        assert!(doc.contains("yav-flow-node"));
        assert!(doc.contains("yav-virtual-list"));
        assert!(doc.contains("yav-rte-surface"));
        assert!(doc.contains("yav:ai-stream"));
        assert!(doc.contains("yav:optimistic-toggle"));
        assert!(doc.contains("yav:optimistic-inc"));
    }

    #[test]
    fn render_srcdoc_xss_protection() {
        let node =
            Node::Element(Element::new("iframe").attr("srcdoc", "<script>alert(1)</script>"));
        let html = render_to_string(&node);
        assert!(!html.contains("<script>"));
        assert!(!html.contains("srcdoc=\"<script"));
        // Double-escaped: entities are re-escaped so the browser's attribute
        // decoding cannot resurrect them into live markup inside srcdoc.
        assert!(html.contains("srcdoc=\"&amp;lt;script"));
    }

    #[test]
    fn render_srcset_xss_protection() {
        let node = Node::Element(Element::new("img").attr("srcset", "javascript:alert(1) 1x"));
        let html = render_to_string(&node);
        assert!(!html.contains("javascript:"));
        assert!(!html.contains("srcset"));
    }

    #[test]
    fn render_style_xss_protection() {
        let node =
            Node::Element(Element::new("div").attr("style", "background:url(javascript:alert(1))"));
        let html = render_to_string(&node);
        assert!(!html.contains("javascript:"));
        assert!(!html.contains("url(javascript"));
        // The dangerous value is refused (neutralised to an empty string).
        assert!(html.contains("style=\"\""));
    }

    #[test]
    fn render_special_attributes_preserve_safe_values() {
        let node = Node::Element(
            Element::new("img")
                .attr("srcset", "a.png 1x, b.png 2x")
                .attr("style", "color: red;"),
        );
        let html = render_to_string(&node);
        assert!(html.contains("srcset=\"a.png 1x, b.png 2x\""));
        assert!(html.contains("style=\"color: red;\""));
    }

    #[test]
    fn render_form_and_usemap_url_protection() {
        let input = Node::Element(Element::new("input").attr("form", "javascript:alert(1)"));
        assert!(!render_to_string(&input).contains("javascript:"));

        let map = Node::Element(Element::new("img").attr("usemap", "javascript:alert(1)"));
        assert!(!render_to_string(&map).contains("javascript:"));

        let safe = Node::Element(Element::new("img").attr("usemap", "#my-map"));
        assert!(render_to_string(&safe).contains("usemap=\"#my-map\""));
    }
}
