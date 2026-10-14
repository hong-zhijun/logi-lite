@main
struct MappingActivationTests {
    static func main() {
        var first = MappingActivationPolicy(defaultPending:true)
        precondition(!first.shouldEnable(ready:false, enabled:false, active:false))
        precondition(first.shouldEnable(ready:true, enabled:false, active:false))
        // A failed input installation must not be retried every state refresh.
        precondition(!first.shouldEnable(ready:true, enabled:false, active:false))
        first.resolveDefault()
        precondition(!first.shouldEnable(ready:true, enabled:false, active:false))

        var paused = MappingActivationPolicy(defaultPending:false)
        precondition(!paused.shouldEnable(ready:true, enabled:false, active:false))
        precondition(!paused.shouldEnable(ready:false, enabled:false, active:false))
        precondition(!paused.shouldEnable(ready:true, enabled:false, active:false))

        // Permission revocation preserves an enabled preference; regrant resumes.
        var resume = MappingActivationPolicy(defaultPending:false)
        precondition(resume.shouldEnable(ready:true, enabled:true, active:false))
        precondition(!resume.shouldEnable(ready:true, enabled:true, active:true))
        precondition(!resume.shouldEnable(ready:false, enabled:true, active:false))
        precondition(resume.shouldEnable(ready:true, enabled:true, active:false))

        // Restart restores an enabled preference, whereas a manual/emergency
        // pause (enabled=false) remains paused even after permissions change.
        var restart = MappingActivationPolicy(defaultPending:false)
        precondition(restart.shouldEnable(ready:true, enabled:true, active:false))
        restart.resolveDefault()
        precondition(!restart.shouldEnable(ready:false, enabled:false, active:false))
        precondition(!restart.shouldEnable(ready:true, enabled:false, active:false))
        print("Mapping activation lifecycle checks passed")
    }
}
