# Impact analysis

CLI impact and desktop Show impact traverse incoming relationships in Rust.
Direct means one hop; transitive means multiple hops. Possible results require
inferred evidence. Exact/resolved paths take precedence. No runtime prediction
or impact score is produced. Selecting a file/class seeds its declarations.
CONTAINS is structural and excluded. Every result retains its evidence path.

Depth defaults to 8 and is capped at 32. Cycle protection uses visited states.
Limits: 20,000 states, 150,000 examined edges and 2,000 result identities.
Truncated results are marked. Filters apply to each hop. Static test selection
cannot guarantee coverage. Git comparison uses baseline graphs for removed symbols.
