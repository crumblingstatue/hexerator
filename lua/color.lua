return function(b)
    local r = b
    local g = b
    local b = b
    return {r % 256, g % 256, b % 256}
end