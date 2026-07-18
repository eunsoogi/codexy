// Hook process timeout, output-bound, and descendant cleanup coverage lives in
// src/validation/hooks/context/process_tests.rs. Keeping those probes at the
// trusted process seam avoids executing modified plugin hooks during validation.
