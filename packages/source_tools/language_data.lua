local export = {}

-- mw.loadData tables are read-only proxies that cannot be serialized; copy them.
-- (Module:table's deepcopy was removed from Wiktionary in 2026.)
local function deepcopy(value)
    if type(value) ~= "table" then return value end
    local copy = {}
    for k, v in pairs(value) do copy[k] = deepcopy(v) end
    return copy
end

-- Exports run one code prefix at a time ({{#invoke:...|languages|ab}}): a single
-- invoke is limited to 60 s. Prefix "~" selects codes not starting with a-z.
local function selected(code, prefix)
    if prefix == nil or prefix == "" then return true end
    if prefix == "~" then return not code:sub(1, 1):match("[a-z]") end
    return code:sub(1, #prefix) == prefix
end

function export.languages_count()
    local n = 0
    for _ in pairs(mw.loadData("Module:languages/data/all")) do n = n + 1 end
    for _ in pairs(mw.loadData("Module:etymology languages/data")) do n = n + 1 end
    return tostring(n)
end

function export.families_count()
    local n = 0
    for _ in pairs(mw.loadData("Module:families/data")) do n = n + 1 end
    return tostring(n)
end

function export.languages(frame)
    local prefix = frame and frame.args[1]
    -- https://en.wiktionary.org/wiki/Module:languages
    local m_languages = require("Module:languages")

    local function getData(code, data, kind)
        local lang = m_languages.getByCode(code, nil, true)
        -- Oldest first. getAncestorChain became a tree in 2026; getAncestorChainOld
        -- keeps the earlier single-line walk, nearest ancestor first.
        local ancestors = {}
        for _, ancestor in ipairs(lang:getAncestorChainOld()) do
            table.insert(ancestors, 1, ancestor:getCode())
        end
        local ret = {
            mainCode = lang:getCode(),
            canonicalName = lang:getCanonicalName(),
            family = lang:getFamilyCode(),
            ---- To get other names, aliases and varieties in one list:
            -- otherNames = lang:getOtherNames(),
            otherNames = lang:getOtherNames(),
            aliases = lang:getAliases(),
            varieties = lang:getVarieties(),
            scripts = lang:getScriptCodes(),
            -- The nearest language that is not an etymology-only language (Wiktionary's
            -- "full" language; getNonEtymologicalCode before 2026). E.g.
            -- for both "VL." (Vulgar Latin) and "ita-ola" (Old Latin) it is
            -- "la". For a regular language it is its own code.
            nonEtymologyOnly = lang:getFullCode(),
            wikidataItem = lang:getWikidataItem(),
            wikipediaArticle = lang:getWikipediaArticle(),
            -- The immediate parent(s) of the language. This will usually be
            -- one language code, but can be multiple in the case of mixture
            -- languages like creoles etc. When it is one code, it should
            -- correspond to the last item in "ancestors".
            parents = lang:getAncestorCodes(),
            -- A list of ancestor language codes from oldest to youngest,
            -- including proto-languages.
            ancestors = ancestors,
            -- regular, appendix-constructed, reconstructed, or etymology-only
            kind = data.type or kind,
        }
        return ret
    end

    local ret = {}

    -- https://en.wiktionary.org/wiki/Module:languages/data/2
    -- https://en.wiktionary.org/wiki/Module:languages/data/3/* where * is a-z
    -- https://en.wiktionary.org/wiki/Module:languages/data/exceptional
    local allData = mw.loadData("Module:languages/data/all")
    for code, data in pairs(allData) do
        if selected(code, prefix) then
            ret[code] = getData(code, data, "regular")
        end
    end

    -- https://en.wiktionary.org/wiki/Module:etymology_languages/data
    local etyData = mw.loadData("Module:etymology languages/data")
    for code, data in pairs(etyData) do
        if selected(code, prefix) then
            ret[code] = getData(code, data, "etymology-only")
        end
    end
    
    ret = deepcopy(ret)

    return mw.text.jsonEncode(ret)

end

local function otherFamilyNames(fam)
    local names, seen = {}, {}
    for _, list in ipairs({fam:getOtherNames(), fam:getAliases(), fam:getVarieties(true)}) do
        for _, name in ipairs(list or {}) do
            if type(name) == "string" and not seen[name] then
                seen[name] = true
                table.insert(names, name)
            end
        end
    end
    return names
end

function export.families(frame)
    local prefix = frame and frame.args[1]
    -- https://en.wiktionary.org/wiki/Module:families
    local m_families = require("Module:families")

    -- https://en.wiktionary.org/wiki/Module:families/data
    local famData = mw.loadData("Module:families/data")
    
    local function getSuperfamilies(fam)
        -- We reverse the order of superfamilies to correspond with the ordering of
        -- lang ancestors, i.e. remotest to nearest (see above).
        local function rev(t)
            local ret = {}
            for i = #t, 1, -1 do
                table.insert(ret, t[i])
            end
            return ret
        end

        local superfamilies = {}
        local superfamily = fam:getFamily()
        while superfamily do
            code = superfamily:getCode()
            for _, a_fam in ipairs(superfamilies) do
                if a_fam == code then
                    return rev(superfamilies)
                end
            end 
            table.insert(superfamilies, code)
            superfamily = superfamily:getFamily()
        end
        return rev(superfamilies)
    end

    local ret = {}

    for code, data in pairs(famData) do
      if selected(code, prefix) then
        local fam = m_families.getByCode(code)
        ret[code] = {
            mainCode = fam:getCode(),
            canonicalName = fam:getCanonicalName(),
            protoLanguage = fam:getProtoLanguageCode(),
            superfamilies = getSuperfamilies(fam),
            -- Families' alternative names moved from otherNames to aliases and
            -- varieties in 2026; keep otherNames as all of them, as before.
            otherNames = otherFamilyNames(fam),
            aliases = fam:getAliases(),
            varieties = fam:getVarieties(true),
            wikidataItem = fam:getWikidataItem(),
            wikipediaArticle = fam:getWikipediaArticle(),
        }
      end
    end
    
    ret = deepcopy(ret)

    return mw.text.jsonEncode(ret)

end

return export