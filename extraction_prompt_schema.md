schema_prompt_full: > Extract all beverage products
  from this page. Return JSON: {"brand_name": string
  (brand name exactly as displayed), "products":
  [{"name": string (full product name including
  flavor and variant e.g. "Hi Boy Blood Orange
  Cardamom 5mg"), "flavor": string (primary flavor
  profile e.g. "Blood Orange Cardamom"),
  "product_line": string or null (sub-brand or
  product line e.g. "Hi Boy", "Hi'er Boy",
  "SuperBloom", "OG", "Amplify"), "product_type": one
  of "seltzer" | "soda" | "tonic" | "cocktail" |
  "spirit_bottle" | "shot" | "cider" | "tea" |
  "lemonade" | "mixer" | "other", "thc_mg": number
  (milligrams of delta-9 THC per serving, 0 if
  CBD-only), "cbd_mg": number (milligrams of CBD per
  serving, 0 if none listed), "other_cannabinoids":
  object (any other cannabinoids with mg values e.g.
  {"CBG": 15, "CBN": 5}, empty object {} if none),
  "functional_ingredients": array of strings
  (adaptogens, mushrooms, nootropics, vitamins,
  caffeine e.g. ["Lion's Mane", "Cordyceps",
  "Reishi"], empty array [] if none), "volume_oz":
  number or null (container size in fluid ounces, 12
  for standard cans, 25.4 for 750ml bottles, 1.7 for
  50ml shots), "format": one of "can" | "slim_can" |
  "bottle" | "shot" | "pouch", "calories": number or
  null (calories per serving if listed),
  "sugar_free": boolean (true if zero sugar or
  sugar-free is explicitly stated), "price_usd":
  number or null (listed retail price in USD),
  "pack_size": number or null (units per listing: 1
  for singles, 4 for 4-packs, etc.),
  "unit_price_usd": number or null (price_usd divided
  by pack_size), "available": boolean (true if in
  stock or Add to Cart is visible, false if sold
  out), "product_url": string or null (full URL to
  the specific product page)}]}. Extract every
  product visible on the page. If a value is not
  found on the page, use null. Do not guess or
  hallucinate values — only extract what is
  explicitly present.
schema_prompt_lite: > Extract all beverage products
  from this page. Return JSON: {"brand_name": string,
  "products": [{"name": string (full product name
  with flavor), "thc_mg": number, "cbd_mg": number,
  "price_usd": number or null, "pack_size": number or
  null, "available": boolean}]}. Only extract what is
  explicitly on the page.
