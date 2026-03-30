package com.traverse.demo.android

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.Card
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContent {
            MaterialTheme {
                TraverseAndroidDemoScreen(sampleStateUpdates)
            }
        }
    }
}

private val sampleStateUpdates = listOf(
    DemoStateUpdate(
        "Discovering capability",
        "Looking up expedition.planning.plan-expedition@1.0.0 in the registry."
    ),
    DemoStateUpdate(
        "Evaluating constraints",
        "Confirming the approved expedition capability can run locally."
    ),
    DemoStateUpdate(
        "Executing workflow",
        "Traversing the approved expedition planning workflow."
    ),
    DemoStateUpdate(
        "Completed",
        "The expedition plan is ready for final review."
    )
)

data class DemoStateUpdate(
    val title: String,
    val detail: String,
)

@Composable
private fun TraverseAndroidDemoScreen(stateUpdates: List<DemoStateUpdate>) {
    Scaffold { padding ->
        LazyColumn(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding),
            contentPadding = PaddingValues(16.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            item {
                Card(modifier = Modifier.fillMaxWidth()) {
                    Column(modifier = Modifier.padding(16.dp)) {
                        Text("Traverse Android Demo", style = MaterialTheme.typography.headlineSmall)
                        Text(
                            "Plan Expedition",
                            style = MaterialTheme.typography.titleMedium,
                            modifier = Modifier.padding(top = 8.dp)
                        )
                        Text(
                            "Runtime states and final trace summary render from the approved expedition demo fixture.",
                            style = MaterialTheme.typography.bodyMedium,
                            modifier = Modifier.padding(top = 8.dp)
                        )
                    }
                }
            }

            items(stateUpdates) { update ->
                Card(modifier = Modifier.fillMaxWidth()) {
                    Column(modifier = Modifier.padding(16.dp)) {
                        Text(update.title, style = MaterialTheme.typography.titleMedium)
                        Text(
                            update.detail,
                            style = MaterialTheme.typography.bodyMedium,
                            modifier = Modifier.padding(top = 6.dp)
                        )
                    }
                }
            }
        }
    }
}
