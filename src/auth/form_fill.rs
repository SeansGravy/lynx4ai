// Iterative login form detection and credential injection
// Uses snapshot refs to find username/password fields and submit buttons
// Handles multi-page login flows (e.g., username page → password page → TOTP page)
//
// Algorithm:
// 1. Take snapshot with filter=interactive
// 2. Find textbox elements with names containing "user", "email", "login"
// 3. Find textbox elements with names containing "pass"
// 4. Fill visible fields, click submit button (or press Enter)
// 5. Re-snapshot — if new fields appear (TOTP, verification), fill those too
// 6. Repeat up to 5 iterations
//
// TODO: implement once click/type_text CDP actions are fully wired
