import "./search.css";
import { Item, Lang, term } from "./types";
import { cleanTerm, searchItems, searchLangs } from "./api";
import { useGoTo } from "./navigate";
import {
  selectedLang,
  setSelectedLang,
  selectedItem,
  selectedTreeKind,
  debounce,
} from "../state";

import {
  For,
  Show,
  createEffect,
  createSignal,
  on,
  onCleanup,
  onMount,
} from "solid-js";

type Option = { kind: "item"; item: Item } | { kind: "lang"; lang: Lang };

// The one input. It reads like a dictionary citation — a small-caps language
// chip followed by the term — and offers words in that language as you type,
// with matching languages in a second group. Typing an exact language name
// followed by a space swaps the chip; Backspace on an empty term, or clicking
// the chip, edits it.
export default function SearchBox() {
  const goTo = useGoTo();
  let inputEl!: HTMLInputElement;
  let rootEl!: HTMLDivElement;

  const [text, setText] = createSignal("");
  const [editingLang, setEditingLang] = createSignal(false);
  const [open, setOpen] = createSignal(false);
  const [items, setItems] = createSignal<Item[]>([]);
  const [langs, setLangs] = createSignal<Lang[]>([]);
  const [active, setActive] = createSignal(-1);

  const options = (): Option[] => {
    // In term mode only offer languages the text is actually a prefix of, so
    // "stat" doesn't dredge up "Tat".
    const q = cleanTerm(text());
    const langOpts: Option[] = langs()
      .filter(
        (l) =>
          l.id !== selectedLang().id &&
          (editingLang() || l.name.toLowerCase().startsWith(q))
      )
      .slice(0, editingLang() ? 8 : 3)
      .map((lang) => ({ kind: "lang", lang }));
    if (editingLang()) return langOpts;
    return [
      ...items().slice(0, 8).map((item): Option => ({ kind: "item", item })),
      ...langOpts,
    ];
  };

  // Show the current word when a tree is loaded from a URL.
  createEffect(
    on(selectedItem, (it) => {
      if (it && document.activeElement !== inputEl) {
        setText(term(it));
        setEditingLang(false);
      }
    })
  );

  let seq = 0;
  const query = debounce(async (raw: string) => {
    const q = cleanTerm(raw);
    const mine = ++seq;
    if (!q) {
      setItems([]);
      setLangs([]);
      return;
    }
    const [foundItems, foundLangs] = await Promise.all([
      editingLang() ? Promise.resolve([]) : searchItems(selectedLang(), q).catch(() => []),
      searchLangs(q).catch(() => []),
    ]);
    if (mine !== seq) return;
    setItems(foundItems);
    setLangs(foundLangs);
    setActive(-1);
    setOpen(true);
  }, 120);

  const setLang = (lang: Lang) => {
    setSelectedLang(lang);
    setEditingLang(false);
    setText("");
    setItems([]);
    setLangs([]);
    setOpen(false);
    inputEl.focus();
  };

  const choose = (opt: Option) => {
    if (opt.kind === "lang") {
      setLang(opt.lang);
      return;
    }
    setText(term(opt.item));
    setOpen(false);
    inputEl.blur();
    goTo(selectedTreeKind(), opt.item);
  };

  const onInput = async (value: string) => {
    setText(value);
    // "latin " → chip, without waiting for the menu.
    if (!editingLang() && value.endsWith(" ") && value.trim()) {
      const name = value.trim().toLowerCase();
      const exact =
        langs().find((l) => l.name.toLowerCase() === name) ??
        (await searchLangs(name).catch(() => [] as Lang[])).find(
          (l) => l.name.toLowerCase() === name
        );
      if (exact && text() === value) {
        setLang(exact);
        return;
      }
    }
    query(value);
  };

  const beginEditingLang = () => {
    setEditingLang(true);
    setText(selectedLang().name);
    setItems([]);
    setOpen(false);
    inputEl.focus();
    requestAnimationFrame(() => inputEl.select());
    query(selectedLang().name);
  };

  const onKeyDown = (e: KeyboardEvent) => {
    const opts = options();
    switch (e.key) {
      case "ArrowDown":
        if (!opts.length) return;
        e.preventDefault();
        setOpen(true);
        setActive((active() + 1) % opts.length);
        break;
      case "ArrowUp":
        if (!opts.length) return;
        e.preventDefault();
        setActive((active() - 1 + opts.length) % opts.length);
        break;
      case "Enter": {
        e.preventDefault();
        const pick = opts[active()] ?? opts.find((o) => o.kind === "item") ?? opts[0];
        if (pick) choose(pick);
        break;
      }
      case "Escape":
        if (editingLang()) {
          setEditingLang(false);
          setText(selectedItem() ? term(selectedItem()!) : "");
        }
        setOpen(false);
        break;
      case "Backspace":
        if (text() === "" && !editingLang()) {
          e.preventDefault();
          beginEditingLang();
        }
        break;
    }
  };

  onMount(() => {
    const onDocKey = (e: KeyboardEvent) => {
      if (
        e.key === "/" &&
        !(e.target instanceof HTMLInputElement) &&
        !(e.target instanceof HTMLTextAreaElement)
      ) {
        e.preventDefault();
        inputEl.focus();
        inputEl.select();
      }
    };
    const onDocPointer = (e: PointerEvent) => {
      if (!rootEl.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener("keydown", onDocKey);
    document.addEventListener("pointerdown", onDocPointer);
    onCleanup(() => {
      document.removeEventListener("keydown", onDocKey);
      document.removeEventListener("pointerdown", onDocPointer);
    });
  });

  const optionId = (i: number) => `search-option-${i}`;

  return (
    <div ref={rootEl} class="search" classList={{ "editing-lang": editingLang() }}>
      <form
        class="search-form"
        role="search"
        onSubmit={(e) => e.preventDefault()}
        onClick={() => inputEl.focus()}
      >
        <button
          type="button"
          class="chip"
          title="change language"
          onClick={(e) => {
            e.stopPropagation();
            beginEditingLang();
          }}
        >
          {selectedLang().name}
        </button>
        <input
          ref={inputEl}
          class="search-input"
          type="text"
          autocomplete="off"
          autocapitalize="off"
          spellcheck={false}
          placeholder={editingLang() ? "language…" : "word…"}
          aria-label={editingLang() ? "language" : `word in ${selectedLang().name}`}
          role="combobox"
          aria-expanded={open() && options().length > 0}
          aria-controls="search-menu"
          aria-autocomplete="list"
          aria-activedescendant={active() >= 0 ? optionId(active()) : undefined}
          value={text()}
          onInput={(e) => onInput(e.currentTarget.value)}
          onKeyDown={onKeyDown}
          onFocus={() => {
            if (options().length) setOpen(true);
          }}
        />
      </form>
      <Show when={open() && options().length > 0}>
        <ul id="search-menu" class="search-menu" role="listbox">
          <For each={options()}>
            {(opt, i) => (
              <li
                id={optionId(i())}
                role="option"
                aria-selected={active() === i()}
                classList={{
                  active: active() === i(),
                  "opt-item": opt.kind === "item",
                  "opt-lang": opt.kind === "lang",
                  "group-start":
                    opt.kind === "lang" &&
                    (i() === 0 || options()[i() - 1].kind === "item"),
                }}
                onPointerEnter={() => setActive(i())}
                onPointerDown={(e) => e.preventDefault()}
                onClick={() => choose(opt)}
              >
                <Show
                  when={opt.kind === "item" && opt}
                  fallback={
                    <>
                      <span class="lang">{(opt as { lang: Lang }).lang.name}</span>
                      <span class="hint">search in this language</span>
                    </>
                  }
                >
                  {(o) => {
                    const it = () => (o() as { item: Item }).item;
                    return (
                      <>
                        <span class="lang">{it().lang.name}</span>{" "}
                        <span class="term">{term(it())}</span>
                        <Show when={it().romanization}>
                          {" "}
                          <span class="romanization">{it().romanization}</span>
                        </Show>
                        <Show when={it().pos?.length}>
                          <span class="gloss">
                            <i>{it().pos![0]}</i>
                            <Show when={it().gloss?.[0]}> · {it().gloss![0]}</Show>
                          </span>
                        </Show>
                      </>
                    );
                  }}
                </Show>
              </li>
            )}
          </For>
        </ul>
      </Show>
    </div>
  );
}
