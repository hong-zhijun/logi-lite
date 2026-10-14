/// Separates the first authorized launch from a user's later explicit pause.
struct MappingActivationPolicy {
    var defaultPending: Bool
    private var attempted = false

    init(defaultPending: Bool) { self.defaultPending = defaultPending }

    mutating func shouldEnable(ready: Bool, enabled: Bool, active: Bool) -> Bool {
        if !ready { attempted = false; return false }
        guard !active, enabled || defaultPending, !attempted else { return false }
        attempted = true
        return true
    }

    mutating func resolveDefault() { defaultPending = false }
}
