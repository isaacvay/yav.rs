//! Live reload server for development.
//!
//! Provides a lightweight SSE (Server-Sent Events) endpoint and serving of the client reload script.

use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;

/// Events broadcasted to connected browser dev clients.
#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ReloadEvent {
    /// Reload page because changes were successfully built.
    Reload,
    /// Clear any active error overlay.
    ClearError,
    /// Display an error overlay modal in the browser.
    Error(String),
    /// Hot-swap CSS stylesheets without full page reload.
    CssReload(String),
}

/// Client JavaScript injected or loaded for live reloading upon file change.
pub const CLIENT_RELOAD_SCRIPT: &str = r#"(function() {
  var url = window.location.protocol + "//" + window.location.hostname + ":{{PORT}}/reload";
  var source = new EventSource(url);

  function removeOverlay() {
    var existing = document.getElementById("yav-error-overlay");
    if (existing) existing.remove();
  }

  function showOverlay(errorData) {
    removeOverlay();
    var overlay = document.createElement("div");
    overlay.id = "yav-error-overlay";
    overlay.style.cssText = "position:fixed;top:0;left:0;right:0;bottom:0;background:rgba(15,23,42,0.92);backdrop-filter:blur(8px);z-index:999999;display:flex;align-items:center;justify-content:center;padding:24px;box-sizing:border-box;font-family:ui-monospace,SFMono-Regular,Menlo,Monaco,Consolas,monospace;";
    
    var card = document.createElement("div");
    card.style.cssText = "background:#1e293b;border:1px solid #ef4444;border-radius:12px;padding:28px;max-width:850px;width:100%;box-shadow:0 25px 50px -12px rgba(0,0,0,0.5);color:#f8fafc;max-height:85vh;overflow-y:auto;";
    
    var header = document.createElement("div");
    header.style.cssText = "display:flex;align-items:center;justify-content:space-between;border-bottom:1px solid #334155;padding-bottom:16px;margin-bottom:20px;";
    var fileLabel = (typeof errorData === "object" && errorData.file) ? String(errorData.file) : "Build Error";
    var leftGroup = document.createElement("div");
    leftGroup.style.cssText = "display:flex;align-items:center;gap:10px;";
    var badge = document.createElement("span");
    badge.style.cssText = "background:#ef4444;color:white;padding:4px 8px;border-radius:6px;font-size:12px;font-weight:bold;";
    badge.textContent = "YAV ERROR";
    var labelSpan = document.createElement("span");
    labelSpan.style.cssText = "color:#94a3b8;font-size:13px;";
    labelSpan.textContent = fileLabel;
    leftGroup.appendChild(badge);
    leftGroup.appendChild(labelSpan);
    var closeBtn = document.createElement("button");
    closeBtn.style.cssText = "background:#334155;color:#94a3b8;border:none;border-radius:6px;padding:4px 10px;cursor:pointer;";
    closeBtn.textContent = "✕";
    closeBtn.onclick = removeOverlay;
    header.appendChild(leftGroup);
    header.appendChild(closeBtn);
    
    var body = document.createElement("pre");
    body.style.cssText = "background:#0f172a;padding:16px;border-radius:8px;font-size:13px;line-height:1.5;overflow-x:auto;color:#fca5a5;white-space:pre-wrap;margin:0;";
    body.textContent = (typeof errorData === "object" && errorData.message) ? errorData.message : errorData;
    
    var footer = document.createElement("div");
    footer.style.cssText = "margin-top:16px;color:#64748b;font-size:12px;display:flex;justify-content:space-between;";
    footer.innerHTML = "<span>Fix the error in your editor and save to reload.</span><span>Yav Dev Server</span>";
    
    card.appendChild(header);
    card.appendChild(body);
    card.appendChild(footer);
    overlay.appendChild(card);
    document.body.appendChild(overlay);
  }

  function morphElement(oldEl, newEl) {
    if (!oldEl || !newEl) return;
    if (oldEl.nodeType !== newEl.nodeType || oldEl.nodeName !== newEl.nodeName) {
      oldEl.replaceWith(newEl.cloneNode(true));
      return;
    }
    if (oldEl.nodeType === 3) {
      if (oldEl.nodeValue !== newEl.nodeValue) {
        oldEl.nodeValue = newEl.nodeValue;
      }
      return;
    }
    var oldAttrs = oldEl.attributes;
    var newAttrs = newEl.attributes;
    if (oldAttrs && newAttrs) {
      for (var i = oldAttrs.length - 1; i >= 0; i--) {
        var attrName = oldAttrs[i].name;
        if (!newEl.hasAttribute(attrName)) {
          oldEl.removeAttribute(attrName);
        }
      }
      for (var j = 0; j < newAttrs.length; j++) {
        var aName = newAttrs[j].name;
        var aVal = newAttrs[j].value;
        if (oldEl.getAttribute(aName) !== aVal) {
          oldEl.setAttribute(aName, aVal);
        }
      }
    }
    if (oldEl.tagName === 'INPUT' || oldEl.tagName === 'TEXTAREA') {
      if (oldEl.type !== 'file' && oldEl.type !== 'password') {
        if (oldEl.value !== newEl.value && document.activeElement !== oldEl) {
          oldEl.value = newEl.value;
        }
      }
      if (oldEl.type === 'checkbox' || oldEl.type === 'radio') {
        oldEl.checked = newEl.checked;
      }
    }
    var oldChildren = Array.from(oldEl.childNodes);
    var newChildren = Array.from(newEl.childNodes);

    // Key-aware matching for stable DOM updates
    var oldKeyMap = {};
    for (var ki = 0; ki < oldChildren.length; ki++) {
      var oc = oldChildren[ki];
      if (oc.nodeType === 1) {
        var kVal = oc.getAttribute('key') || oc.getAttribute('data-key') || (oc.id ? '#' + oc.id : null);
        if (kVal) oldKeyMap[kVal] = oc;
      }
    }

    var commonLen = Math.min(oldChildren.length, newChildren.length);
    for (var k = 0; k < commonLen; k++) {
      var nChild = newChildren[k];
      var nKey = (nChild.nodeType === 1) ? (nChild.getAttribute('key') || nChild.getAttribute('data-key') || (nChild.id ? '#' + nChild.id : null)) : null;
      if (nKey && oldKeyMap[nKey]) {
        morphElement(oldKeyMap[nKey], nChild);
      } else {
        morphElement(oldChildren[k], nChild);
      }
    }
    if (newChildren.length > oldChildren.length) {
      for (var m = commonLen; m < newChildren.length; m++) {
        oldEl.appendChild(newChildren[m].cloneNode(true));
      }
    } else if (oldChildren.length > newChildren.length) {
      for (var n = oldChildren.length - 1; n >= commonLen; n--) {
        oldChildren[n].remove();
      }
    }
  }

  function hotReloadDOM() {
    var scrollX = window.scrollX;
    var scrollY = window.scrollY;
    var activeId = document.activeElement ? document.activeElement.id : null;

    fetch(window.location.href, { cache: 'no-cache' })
      .then(function(res) {
        if (!res.ok) throw new Error("HTTP " + res.status);
        return res.text();
      })
      .then(function(html) {
        var parser = new DOMParser();
        var doc = parser.parseFromString(html, "text/html");
        if (doc.title && document.title !== doc.title) {
          document.title = doc.title;
        }
        morphElement(document.body, doc.body);
        window.scrollTo(scrollX, scrollY);
        if (activeId) {
          var restored = document.getElementById(activeId);
          if (restored) restored.focus();
        }
        removeOverlay();
        console.log("[yav] DOM morphed successfully without page reload.");
      })
      .catch(function(err) {
        console.warn("[yav] Fast morph fallback to full reload:", err);
        window.location.reload();
      });
  }

  source.onmessage = function(event) {
    if (event.data === "reload") {
      console.log("[yav] Change detected, updating DOM...");
      removeOverlay();
      hotReloadDOM();
    } else if (event.data === "clear_error") {
      removeOverlay();
    } else if (event.data.indexOf("css:") === 0) {
      var cssPath = event.data.substring(4);
      console.log("[yav] Hot-swapping stylesheet: " + cssPath);
      removeOverlay();
      var links = document.querySelectorAll('link[rel="stylesheet"]');
      var updated = false;
      for (var i = 0; i < links.length; i++) {
        var link = links[i];
        var href = link.getAttribute("href") || "";
        if (!cssPath || href.indexOf(cssPath) !== -1 || href.indexOf("styles.css") !== -1 || href.indexOf("app.css") !== -1) {
          var cleanHref = href.split("?")[0];
          link.setAttribute("href", cleanHref + "?t=" + Date.now());
          updated = true;
        }
      }
      if (!updated && links.length > 0) {
        for (var j = 0; j < links.length; j++) {
          var l = links[j];
          var h = l.getAttribute("href") || "";
          if (h.indexOf("http") !== 0 || h.indexOf(window.location.host) !== -1) {
            l.setAttribute("href", h.split("?")[0] + "?t=" + Date.now());
          }
        }
      }
    } else if (event.data.indexOf("error:") === 0) {
      var content = event.data.substring(6);
      try {
        showOverlay(JSON.parse(content));
      } catch(e) {
        showOverlay(content);
      }
    }
  };
  source.onerror = function() {
    // Retry automatically handled by EventSource
  };
})();
"#;

/// Live reload server handling SSE connections and file change notifications.
#[derive(Clone)]
pub struct ReloadServer {
    tx: Arc<broadcast::Sender<ReloadEvent>>,
    port: u16,
}

impl ReloadServer {
    /// Starts the reload server on the preferred port, falling back to an ephemeral port if occupied.
    pub async fn start(preferred_port: u16) -> Result<(Self, u16), std::io::Error> {
        let listener = match TcpListener::bind(format!("127.0.0.1:{preferred_port}")).await {
            Ok(l) => l,
            Err(_) => TcpListener::bind("127.0.0.1:0").await?,
        };

        let local_addr = listener.local_addr()?;
        let actual_port = local_addr.port();
        let (tx, _) = broadcast::channel::<ReloadEvent>(16);
        let tx = Arc::new(tx);

        let server = Self {
            tx: tx.clone(),
            port: actual_port,
        };

        let server_clone = server.clone();
        tokio::spawn(async move {
            server_clone.run_listener(listener).await;
        });

        Ok((server, actual_port))
    }

    /// Broadcasts a reload event to all connected browser clients.
    pub fn notify_reload(&self) {
        let _ = self.tx.send(ReloadEvent::Reload);
    }

    /// Broadcasts an error event to display the error overlay in all connected browsers.
    #[allow(dead_code)]
    pub fn notify_error(&self, error_message: impl Into<String>) {
        let _ = self.tx.send(ReloadEvent::Error(error_message.into()));
    }

    /// Broadcasts a signal to dismiss any active error overlay.
    #[allow(dead_code)]
    pub fn notify_clear_error(&self) {
        let _ = self.tx.send(ReloadEvent::ClearError);
    }

    /// Broadcasts a CSS reload event to replace stylesheets without full page reload.
    pub fn notify_css_reload(&self, path: impl Into<String>) {
        let _ = self.tx.send(ReloadEvent::CssReload(path.into()));
    }

    /// Returns the port on which the reload server is listening.
    #[allow(dead_code)]
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Generates an HTML script tag pointing to this reload server.
    #[allow(dead_code)]
    pub fn script_tag(&self) -> String {
        format!(
            r#"<script src="http://127.0.0.1:{}/reload.js"></script>"#,
            self.port
        )
    }

    async fn run_listener(self, listener: TcpListener) {
        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    let tx = self.tx.clone();
                    let port = self.port;
                    tokio::spawn(async move {
                        if let Err(e) = handle_connection(stream, addr, tx, port).await {
                            tracing::debug!("Reload client connection closed: {e}");
                        }
                    });
                }
                Err(e) => {
                    tracing::debug!("Error accepting reload connection: {e}");
                    break;
                }
            }
        }
    }
}

async fn handle_connection(
    mut stream: TcpStream,
    _addr: SocketAddr,
    tx: Arc<broadcast::Sender<ReloadEvent>>,
    port: u16,
) -> Result<(), std::io::Error> {
    let mut buf = [0u8; 1024];
    let n = stream.read(&mut buf).await?;
    if n == 0 {
        return Ok(());
    }

    let req_str = String::from_utf8_lossy(&buf[..n]);
    let req_line = req_str.lines().next().unwrap_or("");

    // Restrict CORS to local loopback origins only so external websites cannot read dev SSE events
    let allowed_origin = req_str
        .lines()
        .find_map(|line| {
            let (name, val) = line.split_once(':')?;
            if name.trim().eq_ignore_ascii_case("origin") {
                let origin = val.trim();
                if origin.starts_with("http://127.0.0.1")
                    || origin.starts_with("http://localhost")
                    || origin.starts_with("http://[::1]")
                {
                    return Some(origin.to_string());
                }
            }
            None
        })
        .unwrap_or_else(|| "http://127.0.0.1:3000".to_string());

    if req_line.contains("/reload.js") {
        let js = CLIENT_RELOAD_SCRIPT.replace("{{PORT}}", &port.to_string());
        let response = format!(
            "HTTP/1.1 200 OK\r\n\
             Content-Type: application/javascript; charset=utf-8\r\n\
             Content-Length: {}\r\n\
             Access-Control-Allow-Origin: {}\r\n\
             Connection: close\r\n\r\n\
             {}",
            js.len(),
            allowed_origin,
            js
        );
        stream.write_all(response.as_bytes()).await?;
        stream.flush().await?;
        return Ok(());
    }

    if req_line.contains("/reload") {
        let headers = format!(
            "HTTP/1.1 200 OK\r\n\
             Content-Type: text/event-stream\r\n\
             Cache-Control: no-cache\r\n\
             Connection: keep-alive\r\n\
             Access-Control-Allow-Origin: {allowed_origin}\r\n\r\n"
        );
        stream.write_all(headers.as_bytes()).await?;
        stream.flush().await?;

        // Send initial heartbeat
        stream.write_all(b": connected\n\n").await?;
        stream.flush().await?;

        let mut rx = tx.subscribe();
        loop {
            tokio::select! {
                recv_res = rx.recv() => {
                    match recv_res {
                        Ok(ReloadEvent::Reload) => {
                            if stream.write_all(b"data: reload\n\n").await.is_err() {
                                break;
                            }
                            if stream.flush().await.is_err() {
                                break;
                            }
                        }
                        Ok(ReloadEvent::ClearError) => {
                            if stream.write_all(b"data: clear_error\n\n").await.is_err() {
                                break;
                            }
                            if stream.flush().await.is_err() {
                                break;
                            }
                        }
                        Ok(ReloadEvent::Error(msg)) => {
                            let safe_msg: String = msg
                                .chars()
                                .map(|c| if c == '\r' || c == '\n' { ' ' } else { c })
                                .collect();
                            let payload = format!("data: error:{safe_msg}\n\n");
                            if stream.write_all(payload.as_bytes()).await.is_err() {
                                break;
                            }
                            if stream.flush().await.is_err() {
                                break;
                            }
                        }
                        Ok(ReloadEvent::CssReload(path)) => {
                            let safe_path: String = path
                                .chars()
                                .filter(|&c| c != '\r' && c != '\n')
                                .collect();
                            let payload = format!("data: css:{safe_path}\n\n");
                            if stream.write_all(payload.as_bytes()).await.is_err() {
                                break;
                            }
                            if stream.flush().await.is_err() {
                                break;
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(broadcast::error::RecvError::Closed) => break,
                    }
                }
                // Check for client disconnect
                read_res = stream.read(&mut buf) => {
                    match read_res {
                        Ok(0) | Err(_) => break,
                        Ok(_) => {}
                    }
                }
            }
        }
        return Ok(());
    }

    let not_found = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n";
    stream.write_all(not_found.as_bytes()).await?;
    stream.flush().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_reload_server_starts_and_serves_js() {
        let (server, port) = ReloadServer::start(0)
            .await
            .expect("Failed to start server");
        assert!(port > 0);

        let mut stream = TcpStream::connect(format!("127.0.0.1:{port}"))
            .await
            .expect("Failed to connect");
        stream
            .write_all(b"GET /reload.js HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .await
            .unwrap();

        let mut response = vec![0u8; 1024];
        let n = stream.read(&mut response).await.unwrap();
        let res_str = String::from_utf8_lossy(&response[..n]);

        assert!(res_str.contains("HTTP/1.1 200 OK"));
        assert!(res_str.contains("application/javascript"));
        assert!(res_str.contains("EventSource"));

        server.notify_reload();
        server.notify_css_reload("/public/styles.css");
        server.notify_error("test error");
        server.notify_clear_error();
    }
}
