use crate::classifier::LineKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScopeKind {
    Fortran,
    Fypp,
}

struct ScopeEntry {
    #[allow(dead_code)]
    kind: ScopeKind,
    depth: usize,
    /// The name of the scope (e.g., subroutine name, module name).
    name: Option<String>,
}

/// Tracks indentation depth based on line classifications.
pub struct ScopeTracker {
    depth: usize,
    stack: Vec<ScopeEntry>,
    /// Name of the most recently popped (closed) scope.
    last_closed_name: Option<String>,
    /// Whether we are inside a `contains` block (for blank line rules).
    in_contains: bool,
    /// Depth of the most recent `contains` scope.
    contains_depth: Option<usize>,
}

impl Default for ScopeTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl ScopeTracker {
    pub fn new() -> Self {
        Self {
            depth: 0,
            stack: Vec::new(),
            last_closed_name: None,
            in_contains: false,
            contains_depth: None,
        }
    }

    pub fn current_depth(&self) -> usize {
        self.depth
    }

    /// Returns the name of the most recently popped scope, if any.
    pub fn last_closed_name(&self) -> Option<&str> {
        self.last_closed_name.as_deref()
    }

    /// Returns true if we are inside a `contains` block.
    pub fn in_contains(&self) -> bool {
        self.in_contains
    }

    /// Process a line kind, update internal state, and return the indent depth for THIS line.
    pub fn process(&mut self, kind: LineKind) -> usize {
        self.process_with_name(kind, None)
    }

    /// Process a line kind with an optional scope name for block openers.
    pub fn process_with_name(&mut self, kind: LineKind, name: Option<String>) -> usize {
        // Clear last closed name on each new line processing
        self.last_closed_name = None;

        match kind {
            // Block openers: return current depth for this line, then push and increment.
            LineKind::FortranBlockOpen => {
                let line_depth = self.depth;
                self.stack.push(ScopeEntry {
                    kind: ScopeKind::Fortran,
                    depth: line_depth,
                    name,
                });
                self.depth += 1;
                line_depth
            }
            LineKind::FyppBlockOpen => {
                let line_depth = self.depth;
                self.stack.push(ScopeEntry {
                    kind: ScopeKind::Fypp,
                    depth: line_depth,
                    name: None,
                });
                self.depth += 1;
                line_depth
            }

            // Block closers: pop the stack, return the popped depth.
            LineKind::FortranBlockClose => {
                if let Some(entry) = self.stack.pop() {
                    self.depth = entry.depth;
                    self.last_closed_name = entry.name;
                    // If we pop back to or below the contains depth, we're no longer in contains
                    if let Some(cd) = self.contains_depth {
                        if entry.depth <= cd {
                            self.in_contains = false;
                            self.contains_depth = None;
                        }
                    }
                    entry.depth
                } else {
                    eprintln!("ffmt: warning: unmatched FortranBlockClose at depth 0");
                    0
                }
            }
            LineKind::FyppBlockClose => {
                if let Some(entry) = self.stack.pop() {
                    self.depth = entry.depth;
                    entry.depth
                } else {
                    eprintln!("ffmt: warning: unmatched FyppBlockClose at depth 0");
                    0
                }
            }

            // Continuations (else, case, #:elif, #:else): return enclosing scope depth (one less
            // than current), but don't change current depth.
            LineKind::FortranContinuation | LineKind::FyppContinuation => {
                if self.depth > 0 {
                    self.depth - 1
                } else {
                    0
                }
            }

            // Preprocessor continuation (#else, #elif): return current depth, no state change.
            LineKind::PreprocessorContinuation => self.depth,

            // Contains: pop back to enclosing scope depth for this line,
            // set depth = enclosing + 1 for subsequent lines.
            LineKind::FortranContains => {
                if let Some(entry) = self.stack.last() {
                    self.in_contains = true;
                    self.contains_depth = Some(entry.depth);
                    // This line is at enclosing depth; subsequent lines stay at enclosing + 1.
                    // Current depth was enclosing + 1, so we keep it (depth remains unchanged).
                    entry.depth
                } else {
                    0
                }
            }

            // Preprocessor directives and close: return current depth, no state change.
            LineKind::PreprocessorDirective | LineKind::PreprocessorClose => self.depth,

            // All others: return current depth, no state change.
            LineKind::FortranStatement
            | LineKind::FyppStatement
            | LineKind::InlineFypp
            | LineKind::Directive
            | LineKind::Comment
            | LineKind::Blank => self.depth,
        }
    }
}
