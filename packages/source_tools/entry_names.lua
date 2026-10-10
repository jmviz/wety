local export = {}

-- Wiktionary's page title (entry name) for each (language code, term) in an input
-- chunk, as its links compute it: Language:stripDiacritics, e.g. "la", "mātrīx"
-- -> "matrix". The driver stores each chunk as a module returning a JSON string of
-- [[code, term], ...]; the result lists only the terms whose title differs.
function export.entry_names(frame)
    local chunk = frame.args[1]
    local input = mw.text.jsonDecode(require("Module:wety-entry-names/input/" .. chunk))
    local m_languages = require("Module:languages")
    local langs, titles, failed = {}, {}, 0
    for _, pair in ipairs(input) do
        local code, term = pair[1], pair[2]
        local lang = langs[code]
        if lang == nil then
            lang = m_languages.getByCode(code, nil, true) or false
            langs[code] = lang
        end
        if lang then
            local strip = lang.stripDiacritics or lang.makeEntryName
            local ok, title = pcall(strip, lang, term)
            if not ok then
                failed = failed + 1
            elseif title and title ~= term then
                table.insert(titles, {code, term, title})
            end
        end
    end
    return mw.text.jsonEncode({titles = titles, failed = failed})
end

return export
