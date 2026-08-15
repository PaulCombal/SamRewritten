# Translations

SamRewritten's GUI is localised with gettext via GLib's `g_dgettext` (no extra
Rust crate). The English source string is the catalogue key.

## Want to translate SamRewritten?

You don't need to be a programmer. The easiest way by far is
**[Weblate](https://hosted.weblate.org/engage/samrewritten/)**: pick your
language, type your translations in the browser, and they reach us on their own
— no file to download, no GitHub account, no pull request. Weblate also shows
you where each phrase appears in the app and flags the ones that changed.

If your language isn't listed there yet, Weblate can create it: use *Start new
translation* on the project page.

The rest of this section covers the manual route, for when you'd rather work
offline in a `.po` editor.

### 1. Get the file

- **New language:** make a copy of `po/samrewritten.pot` and rename it
  `po/<code>.po`, where `<code>` is your language code — `de` for German, `es`
  for Spanish, `pt_BR` for Brazilian Portuguese, and so on.
- **Improving an existing language:** just open the `.po` file that's already
  there (for example `po/fr.po`).

### 2. Fill in the translations

Open the file in any text editor, or — easier — in a free tool made for this
like [Poedit](https://poedit.net). You'll see pairs like:

```
msgid "Refresh app list"
msgstr ""
```

The `msgid` line is the original English — **leave it exactly as it is**. Type
your translation between the quotes on the `msgstr` line:

```
msgid "Refresh app list"
msgstr "Actualiser la liste des apps"
```

A few things to keep in mind:

- Keep anything inside curly braces (like `{count}`) and any tags (like `<b>`
  or `<a href="…">`) **unchanged** — the app fills those in.
- It's fine to leave a `msgstr ""` empty if you're unsure; it simply falls back
  to English.

### 3. Send it to us

- **Easiest:** upload it to
  [Weblate](https://hosted.weblate.org/engage/samrewritten/) from your language's
  page — it merges your file with whatever is already there.
- **No account:** open a
  [new issue](https://github.com/PaulCombal/SamRewritten/issues) and attach your
  `.po` file. We'll handle the rest.
- **If you're comfortable with GitHub:** fork the project, add your file under
  `po/`, and open a pull request.

That's it — thank you! 🎉

### (Optional) Preview your work

If you build SamRewritten from source, your translation is compiled
automatically on the next build, so you can see it live:

```sh
LANGUAGE=de LANG=de_DE.UTF-8 cargo run
```

(Replace `de` with your language code.)

## For developers — marking strings

```rust
use crate::gui_frontend::i18n::{tr, tr_noop, trn};

label.set_text(&tr("Refresh app list"));        // translate now
let opts = [(tr_noop("Light"), "light")];       // mark at definition,
let item = MenuItem::new(Some(tr(opts[0].0)));  //   translate at use
trn("1 app", "{n} apps", n);                    // plural (insert n yourself)
```

Do **not** wrap dynamic data from Steam (achievement/app names) — only the
app's own chrome. Keep each translatable string on one physical line; for
multi-paragraph text, use one `tr()` per paragraph and join them in code, so the
catalogue keys stay clean and stable.

After adding/changing strings, refresh the template:

```sh
./po/update-pot.sh        # needs gettext (xgettext)
```

This rewrites `po/samrewritten.pot` only. The `po/<lang>.po` files belong to
[Weblate](https://hosted.weblate.org/projects/samrewritten/samrewritten/), which
merges the new template into each of them and pushes the result back — so don't
edit them by hand, or your commit and Weblate's will conflict over the same
lines. If you do need to merge locally (Weblate down, or an offline release),
lock the component first with `wlc lock` and pass `--merge`.

`build.rs` compiles each `po/<lang>.po` to
`locale/<lang>/LC_MESSAGES/samrewritten.mo` on the next `cargo build`, so a dev
run from the source tree picks it up automatically (the `./locale` branch in
`src/gui_frontend/i18n.rs`).

List every source file with translatable strings in `POTFILES.in`.

## Packaging

Install the compiled `.mo` files into the platform's locale prefix and point
`i18n::locale_dir()` at it. The existing branches cover:

| Target   | Directory                             |
|----------|---------------------------------------|
| AppImage | `$APPDIR/usr/share/locale`            |
| Snap     | `$SNAP/usr/share/locale`              |
| System   | `/usr/share/locale` (gettext default) |
| Override | `$SAM_LOCALE_DIR_FALLBACK`            |
