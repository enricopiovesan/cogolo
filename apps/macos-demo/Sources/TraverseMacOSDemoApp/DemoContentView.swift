import SwiftUI

struct DemoSessionViewModel {
    let session: DemoSession

    static func sample() -> DemoSessionViewModel {
        DemoSessionViewModel(session: DemoSessionRepository.sample())
    }
}

struct DemoContentView: View {
    let viewModel: DemoSessionViewModel

    var body: some View {
        NavigationSplitView {
            List(viewModel.session.stateUpdates) { update in
                VStack(alignment: .leading, spacing: 6) {
                    Text(update.title)
                        .font(.headline)
                    Text(update.detail)
                        .font(.subheadline)
                        .foregroundStyle(.secondary)
                    Text(update.timestamp)
                        .font(.caption.monospaced())
                        .foregroundStyle(.tertiary)
                }
                .padding(.vertical, 6)
            }
            .navigationTitle(viewModel.session.title)
        } detail: {
            ScrollView {
                VStack(alignment: .leading, spacing: 18) {
                    GroupBox("Runtime Summary") {
                        VStack(alignment: .leading, spacing: 10) {
                            labeledRow("Status", value: viewModel.session.status)
                            labeledRow("Request", value: viewModel.session.requestID)
                            labeledRow("Execution", value: viewModel.session.executionID)
                            labeledRow("Trace", value: viewModel.session.traceID)
                            labeledRow("Target", value: viewModel.session.trace.placement.selectedTarget)
                            Text(viewModel.session.summary)
                                .font(.body)
                        }
                        .frame(maxWidth: .infinity, alignment: .leading)
                    }

                    GroupBox("Request Goal") {
                        Text(viewModel.session.request.goal)
                            .frame(maxWidth: .infinity, alignment: .leading)
                    }

                    GroupBox("Emitted Events") {
                        VStack(alignment: .leading, spacing: 8) {
                            ForEach(viewModel.session.trace.emittedEvents, id: \.self) { eventID in
                                Text(eventID)
                                    .font(.body.monospaced())
                            }
                        }
                        .frame(maxWidth: .infinity, alignment: .leading)
                    }

                    GroupBox("Final Output") {
                        VStack(alignment: .leading, spacing: 10) {
                            labeledRow("Plan ID", value: viewModel.session.trace.output.planID)
                            labeledRow("Route", value: viewModel.session.trace.output.route)
                            labeledRow("Weather", value: viewModel.session.trace.output.weatherSummary)
                            labeledRow("Team Status", value: viewModel.session.trace.output.teamStatus)
                            labeledRow("Next Action", value: viewModel.session.trace.output.nextAction)
                        }
                        .frame(maxWidth: .infinity, alignment: .leading)
                    }
                }
                .padding(24)
            }
        }
    }

    @ViewBuilder
    private func labeledRow(_ label: String, value: String) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(label)
                .font(.caption)
                .foregroundStyle(.secondary)
            Text(value)
                .font(.body)
        }
    }
}
