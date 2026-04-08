import SwiftUI

@main
struct TraverseMacOSDemoApp: App {
    var body: some Scene {
        WindowGroup("Traverse macOS Demo") {
            DemoContentView(viewModel: DemoSessionViewModel.sample())
                .frame(minWidth: 1080, minHeight: 720)
        }
        .windowStyle(.titleBar)
    }
}
