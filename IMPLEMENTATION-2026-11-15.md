# Implémentation des correctifs d'audit — 15 novembre 2026

Ce document fait suite à [`AUDIT-2026-11-13.md`](AUDIT-2026-11-13.md).
Il décrit **ce qui a réellement été modifié**, **ce qui a été vérifié**, et
**ce qui reste une limite assumée**. Aucune affirmation de réussite n'est
avancée sans commande de vérification correspondante.

Workspace : 9 crates, MSRV déclaré et vérifié **Rust 1.88**, version `0.1.5`
(`0.1.6 - Unreleased` dans le CHANGELOG).

---

## 1. Sécurité (P1)

| Sujet | Correctif | Fichier |
|---|---|---|
| XSS PPR `</script>` | Les deux chemins (page complète et chunks streaming) sérialisent le HTML de slot via `streaming::escape_js_string_literal`; `render_complete()` injecte aussi le nonce CSP. Tests de non-régression. | `ppr.rs`, `streaming.rs` |
| SSRF Web Push | Validation syntaxique + DNS avant signature (https seul, refus userinfo/fragment/control chars, refus IP privées/loopback/link-local/multicast), `redirect::Policy::none()`, timeout 10 s, nouvelle variante `WebPushError::UnsafeEndpoint`. | `web_push.rs` |
| VAPID JWT | Claims construits avec `serde_json::json!` au lieu de `format!`. | `web_push.rs` |
| RFC 8291 / 8188 | Dérivation HKDF en deux étapes conforme (sel lié, contexte vide pour CEK/nonce), délimiteur de dernier enregistrement `0x02`. | `web_push.rs` |
| CSP | Retrait de `https:` de `script-src`, repli sans nonce durci en `script-src 'self'`. | `middleware.rs` |
| WebAuthn | Le helper sans vérification de signature est devenu privé (`verify_ceremony_metadata`) ; seule l'API avec vérification est publique. | `passkeys.rs` |
| Fédération | `forward_auth = false` par défaut, allowlist d'hôtes (refus par défaut), assainissement des fragments activé par défaut. Contournement proxy loopback avec `.no_proxy()` et déclenchement du HTML de repli lors de code d'erreur HTTP amont (`status >= 400`). | `federation.rs` |
| PPR Slots & Cache | Résolution concurrente des slots en parallèle (`join_all`), streaming des slots dynamiques dans le désordre (`FuturesUnordered` + bounded channel) sans blocage. Caches `SHELL_CACHE` et `ComponentCache` bornés avec éviction FIFO. | `ppr.rs` |
| WASM Island CSP Nonce | Support des nonces CSP via `with_nonce(...)` et injection `<script nonce="...">` pour CSP strict sans `unsafe-inline`. | `wasm.rs` |
| AI Token Streaming & RAG | Streaming SSE en temps réel (`stream_text_channel`, `stream_text_stream`) sans pré-tamponnage mémoire des tokens LLM. `VectorIndex` borné (`with_capacity`) et recherche par sélection partielle $O(N + K \log K)$. | `ai.rs` |
| Injection playground GraphQL | `graphiql_html(endpoint)` n'interpole plus l'endpoint dans `fetch('…')` : sérialisation via `escape_js_string_literal` (échappement `<`, `>`, `&`, `/`, U+2028/U+2029). Tests avec endpoint hostile. Docs corrigées (page artisanale, pas GraphiQL 3.x). | `graphql.rs` |
| Échappement contextuel | `escape_srcdoc`, `escape_srcset`, `escape_style` ajoutés et câblés dans le renderer ; JSON-LD encode `<`, `>`, `&` et séparateurs JS ; `org_chart` utilise `escape_url`. | `escape.rs`, `render.rs`, `meta.rs`, `component.rs`, `org_chart.rs` |
| Flash cookies | Rejet des payloads historiques sans `expires_at`, secret éphémère CSPRNG unique (`OnceLock`), plus de panic en fonctionnement normal. | `flash.rs` |
| CSRF | Plus de repli silencieux sur `DEFAULT_DEV_SESSION_SECRET` en production : erreur explicite et « fail closed ». | `forms.rs` |
| Docs OpenAPI | Pages Scalar/Swagger réellement autonomes, sans CDN ni JavaScript ; tests l'assertissant. | `openapi.rs` |

**Contrôle croisé** : tous ces points ont été revérifiés par recherche dans le
code après coup. Un seul lot (GraphQL) n'avait pas été appliqué par les
sous-agents ; il a été corrigé manuellement pendant cette phase de livraison.

---

## 2. Véracité produit (P2)

- `EmbeddedDatabaseAdapter` → `MockDatabaseAdapter`, `LocalLlmAdapter` →
  `MockLlmAdapter` (alias dépréciés conservés) : erreurs explicites au lieu de
  faux `Ok(1)` et de complétions fabriquées.
- `metrics_dashboard_handler` lit désormais `global_tracer()` (spans, erreurs,
  uptime, durée). Les valeurs non instrumentées valent `0`, jamais inventées.
- `geo_ip` : 27 taux de TVA UE, calcul en arithmétique entière exacte.
- `ImagePipeline` et `AppState` ajoutés et ré-exportés ; la doc dit
  explicitement que l'encodage AVIF n'est pas pris en charge.
- `MockServer::delay_ms` réellement consommé (test de latence).
- `email`, `security_audit` : documentation des limites réelles (transport
  `.eml` local, score heuristique local et non certification OWASP).
- Commande CLI `yav test` : support complet du transfert d'arguments et de filtres vers `cargo test` (`trailing_var_arg = true, allow_hyphen_values = true`) dans `yav-cli/src/main.rs` et `yav-cli/src/test_cmd.rs`, avec tests unitaires de parsing dans `crates/yav-cli/src/main.rs`.

---

## 3. Documentation et métadonnées (P0)

- `README.md` : section « Feature maturity », Quick Start compilable,
  avertissement sur les métadonnées de la `0.1.5` publiée sur crates.io
  (`rust-version = 1.75`, families d'icônes), corrections GraphiQL / AVIF /
  multi-régions / slots parallèles / `yav codegen --ts`.
- `SECURITY.md`, `CONTRIBUTING.md` : CSRF activé par défaut, comportement réel
  de `SESSION_SECRET`, écart MSRV crates.io.
- `docs/security.md` : exemples JWT / TOTP / audit / webhooks réécrits avec
  les vraies signatures.
- `docs/limitations.md` : créé, puis mis à jour après les derniers correctifs
  (dashboard réel, doctests, limitation `cargo doc` sur Windows/macOS).
- `docs/architecture.md`, `docs/production.md`, `docs/cli.md`,
  `docs/routing.md` : attributions de crates, numérotation, GraphiQL 3.x,
  AVIF et hydratation séquentielle.
- `CHANGELOG.md` : `0.1.6 - Unreleased` détaillé, `0.1.5` datée.
- `editors/vscode/package.json` et sa copie embarquée : version `0.1.5`,
  URL du dépôt corrigée.
- `assets/` : scripts sans chemin Windows absolu, `ATTRIBUTION.md`,
  `requirements.txt` (numpy / opencv-python / Pillow, imports réels).
- `.env.example`, `crates/yav-icons/LICENSE-*` + `NOTICE.md`,
  `crates/yav-cli/tests/`, `crates/yav-server/tests/`.

---

## 4. Build, dépendances et CI (P4)

- `reqwest` en `default-features = false` + `rustls-tls` (plus d'OpenSSL
  natif silencieux), `futures-util` centralisé, dépendances inutilisées et
  feature `full` factice retirées.
- Retrait de `panic = "abort"` du profil release du workspace **et** de
  `test-app` (un panic de bibliothèque ne doit pas tuer tout le processus).
- Nouveaux fichiers : `rustfmt.toml`, `clippy.toml`, `deny.toml`,
  `.github/dependabot.yml`.
- `.github/workflows/ci.yml` : `--locked` / `--all-targets` partout,
  `--all-features`, doctests, `cargo doc --lib` (Linux uniquement, voir §6),
  contrôle des licences empaquetées, contrôle MSRV, `cargo-audit --locked`,
  vérification des deux projets de démo.
- Doctests : les 10 exemples `yav` sont passés de `ignore` à `rust,no_run`
  (compilés mais non exécutés) ; deux exemples invalides ont été réparés.
  Les 11 exemples de `yav-macros` restent `ignore` **avec justification
  structurelle documentée** (crate proc-macro, fichiers du projet consommateur
  requis). La couverture compilée réelle des macros se trouve dans `yav`.

---

## 5. Vérification

Toutes les commandes ci-dessous ont été exécutées après l'ensemble des
modifications de code et de documentation, sur Windows avec
`rustc 1.98.1` (et `1.88.0` pour le job MSRV).

| Commande | Code | Détail |
|---|---|---|
| `cargo fmt --all -- --check` | **0** | aucun diff (0 ligne) |
| `cargo check --workspace --all-targets --locked` | **0** | 9 crates |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | **0** | aucun lint |
| `cargo test --workspace --all-features --locked` | **0** | 23 suites, **741 tests réussis**, 0 échec, 11 ignorés |
| `cargo test --workspace --doc --locked` | **0** | 8 crates, **53 doctests réussis**, 0 échec, 11 ignorés (`yav-macros`) |
| `cargo doc -p yav-core --no-deps --lib --locked` | **0** | doc ciblée construite (voir §6.2 pour le workspace complet) |
| `cargo +1.88.0 check --workspace --locked` | **0** | MSRV réel confirmé |
| `cargo check --manifest-path examples/dashboard/Cargo.toml --locked` | **0** | lockfile régénéré hors ligne |
| `cargo test --manifest-path examples/dashboard/Cargo.toml --locked` | **0** | 1 test, adapté au CSRF activé par défaut |
| `cargo check --manifest-path test-app/Cargo.toml --locked` | **0** | lockfile régénéré hors ligne |
| `cargo package --list -p yav-icons --allow-dirty` | **0** | `LICENSE-LUCIDE`, `LICENSE-MYNAUI`, `LICENSE-PHOSPHOR`, `NOTICE.md` empaquetés |

### 5.1 Détail des doctests

| Crate | Réussis | Ignorés |
|---|---|---|
| `yav` | 15 | 0 |
| `yav_core` | 9 | 0 |
| `yav_icons` | 5 | 0 |
| `yav_router` | 10 | 0 |
| `yav_server` | 7 | 0 |
| `yav_ui` | 7 | 0 |
| `yav_build` | 0 | 0 |
| `yav_macros` | 0 | **11** (illustratifs, justification en §4/§6.4) |

### 5.2 Échecs rencontrés puis corrigés pendant la livraison

1. **2 doctests `yav` invalides** révélés par le passage `ignore` → `no_run` :
   `TiltPlate` utilisait la prop inexistante `glare` (champ réel `spotlight`) et
   l'exemple `export_static` était un fragment sans contexte. Réparés.
2. **11 doctests `yav-macros`** : le passage forcé en `no_run` produisait 11
   erreurs de compilation (`cannot find macro`, `cannot find crate yav`).
   Retour à `ignore` **avec justification structurelle documentée** ; la
   couverture compilée réelle des macros reste dans `yav` (15 doctests).
3. **Injection endpoint GraphQL** : lot de sous-agent non appliqué, détecté par
   contrôle croisé puis corrigé avec test de non-régression.
4. **`Cargo.lock` et lockfiles des démos** désynchronisés après les changements
   de dépendances : régénérés avec `--offline`, puis revérifiés avec `--locked`.
5. **Test du dashboard en 403** : le POST de `/settings` sans jeton est
   désormais rejeté par CSRF (comportement voulu). Le test a été enrichi :
   il asserts le `403` par défaut **et** le `200` avec
   `CsrfConfig::disabled()` explicite.
6. **`cargo doc --workspace` sur Windows** : collisions de noms de fichiers
   insensibles à la casse (`fn.Card.html` / `fn.card.html`) + collision
   bin/lib `yav`. CI restreinte au Linux avec `--lib`, limitation documentée.
7. **`panic = "abort"`** encore présent dans `test-app` : retiré.
8. **Lien rustdoc `CsrfConfig::disabled`** non résolu : corrigé par chemin
   complet.
9. **Référence `assets/requirements.txt`** inexistante : fichier créé avec les
   imports réels (`numpy`, `opencv-python`, `Pillow`).

---

## 6. Limites assumées et non corrigées localement

1. **Métadonnées crates.io `0.1.5`** : la version publiée annonce encore
   `rust-version = 1.75` et active les trois familles d'icônes. On ne peut pas
   modifier une version déjà publiée ; seule la sortie de `0.1.6` corrigera
   cela. La CI vérifie désormais la cohérence de `rust-version`.
2. **`cargo doc` sur Windows/macOS** : `yav-cli` (binaire `yav`) et `yav`
   (bibliothèque `yav`) entrent en collision sur `target/doc/yav/index.html`,
   et les paires de composants `Card`/`card` produisent des noms de fichiers
   rustdoc qui ne diffèrent que par la casse. Sur les systèmes de fichiers
   insensibles à la casse, rustdoc échoue avec `output filename collision` /
   `Access is denied`. La CI ne construit donc la doc que sur Linux et avec
   `--lib`. Localement, utiliser WSL/Linux ou `cargo doc -p <crate> --lib`.
3. **Warnings rustdoc préexistants** : ~190 liens ambigus (« `X` is both a
   function and a struct ») dus aux paires struct/fonction des composants UI,
   plus quelques liens non résolus hérités. Ils ne font pas échouer la CI
   (`cargo doc` sans `-D warnings`) et n'ont pas été introduits par cet audit.
4. **11 doctests `yav-macros`** restent illustratifs (voir §4).
5. **Warnings `Access is denied` sur les verrous incrémentaux** : artefacts
   de l'environnement Windows observés pendant les builds, sans effet sur les
   codes de sortie.
6. **Aucun commit git** n'a été créé : le dépôt reste « sale » ; la revue et
   le commit sont laissés à l'utilisateur.

---

## 7. Fichiers livrés

- Rapport d'audit : `AUDIT-2026-11-13.md`
- Rapport d'implémentation : `IMPLEMENTATION-2026-11-15.md` (ce fichier)
- Journal des versions : `CHANGELOG.md`
- Limites : `docs/limitations.md`
- Politique de sécurité : `SECURITY.md`
- **100 fichiers suivis modifiés** (11 310 insertions / 2 224 suppressions) et
  **19 fichiers/dossiers nouveaux** (configuration CI, licences d'icônes,
  tests d'intégration, `.env.example`, documentation, rapports).
- Aucun commit créé : le dépôt reste en état « dirty » pour revue.
