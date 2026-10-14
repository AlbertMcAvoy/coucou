package com.coucou.android.core

import com.coucou.android.mochi.BotState

/**
 * What TalkBack says for a Widget: whole sentences for the parts that are tapped, built from the same words the Widget shows (agent
 * names, state words, counts), so the spoken text can never say more than the screen does. Pure.
 */
object WidgetSpoken {
    /** Mochi's mood in plain words ("waiting for you"), not the engine's key ("approval"). */
    fun mood(state: BotState): String = when (state) {
        BotState.IDLE -> "calm"
        BotState.WORKING -> "working"
        BotState.THINKING -> "thinking"
        BotState.SEARCHING -> "searching"
        BotState.APPROVAL -> "waiting for you"
        BotState.QUESTION -> "has a question"
        BotState.ERROR -> "in trouble"
        BotState.FINISHED -> "happy"
        BotState.RATELIMIT -> "at a limit"
        BotState.SLEEPING -> "asleep"
        BotState.DIZZY -> "dizzy"
    }

    /** One agent: "Claude Code, waiting for your approval". */
    fun tile(t: WidgetTile): String = SpokenLabels.agent(t.name, t.stateWord, null)

    /** The tapped card of Solo and Mood: the top agent, or the headline when there is none; with what waits for you. */
    fun top(c: WidgetContent): String {
        val base = c.top?.let { tile(it) } ?: SpokenLabels.agent(c.headline, c.emptyLine.orEmpty(), null)
        return if (c.waiting > 0 && c.top != null) "$base. ${c.waiting} waiting for you" else base
    }

    /** Waiting Alert: "1 waiting. Claude Code", "All calm. Nothing needs you", "Not connected. Waiting for your PC". */
    fun alert(c: WidgetContent): String = "${c.alertLine}. ${c.alertSub}"

    /** The header of the list. */
    fun list(c: WidgetContent): String = if (c.waiting > 0) "${c.headline}. ${c.waiting} waiting for you" else c.headline
}
