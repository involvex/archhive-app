package com.archhive.app

import android.content.Context
import androidx.startup.Initializer
import androidx.window.core.ExperimentalWindowApi
import androidx.window.embedding.RuleController

/**
 * Registers ArcHive's Activity Embedding split rules at app startup so the
 * in-app viewer (ViewerActivity) can display side-by-side with MainActivity
 * on Android 12L+ tablets and foldables. Rules live in res/xml/main_split_config.xml.
 */
@OptIn(ExperimentalWindowApi::class)
class SplitInitializer : Initializer<RuleController> {
    override fun create(context: Context): RuleController {
        return RuleController.getInstance(context).apply {
            setRules(RuleController.parseRules(context, R.xml.main_split_config))
        }
    }

    override fun dependencies(): List<Class<out Initializer<*>>> {
        return emptyList()
    }
}
