#pragma once

#include "../studio_theme.hpp"

#include <QString>

// Stylesheet for the visual chat, expanded from the active Studio theme
// (see studio_theme.hpp). Scoped by object names so it never leaks.
inline QString agentChatStyleSheet(const StudioTheme &theme)
{
    return ThemeManager::expand(QStringLiteral(R"(
        #agentChat, #chatContent, #chatView, #chatWelcome, #chatFooter { background-color: @background@; }
        #chatHeader QLabel, #usageBar, #chatSettings, #composer QLabel { background: transparent; }
        #agentChat QScrollBar:vertical { background: transparent; width: 9px; margin: 2px; }
        #agentChat QScrollBar:horizontal { background: transparent; height: 9px; margin: 2px; }
        #agentChat QScrollBar::handle { background: @borderStrong@; border-radius: 3px; min-height: 24px; min-width: 24px; }
        #agentChat QScrollBar::handle:hover { background: @textFaint@; }
        #agentChat QScrollBar::add-line, #agentChat QScrollBar::sub-line { width: 0; height: 0; border: 0; }
        #agentChat QScrollBar::add-page, #agentChat QScrollBar::sub-page { background: transparent; }
        #chatHeader { background-color: @surface@; border-bottom: 1px solid @border@; }
        #chatTitle { color: @text@; font-size: 15px; font-weight: 700; }
        #chatSubtitle { color: @textMuted@; font-size: 12px; }
        #chatHeaderButton { background-color: transparent; border: 1px solid @borderStrong@; border-radius: 14px;
            padding: 5px 12px; color: @text@; }
        #chatHeaderButton:hover { border-color: @accent@; color: @selectedText@; }

        #usagePill { background-color: @surfaceRaised@; border: 1px solid @border@; border-radius: 11px; }
        #usageLabel { color: @textMuted@; font-size: 11px; }
        #usageValue { color: @text@; font-size: 11px; font-weight: 600; }
        #usageSpend { color: @textMuted@; font-size: 11px; }
        #usageMeter { background-color: @border@; border: 0; border-radius: 2px; }
        #usageMeter::chunk { background-color: @success@; border-radius: 2px; }
        #usageMeter[level="medium"]::chunk { background-color: @warning@; }
        #usageMeter[level="high"]::chunk { background-color: @danger@; }

        #welcomeTitle { color: @text@; font-size: 20px; font-weight: 700; }
        #welcomeSubtitle { color: @textMuted@; font-size: 13px; }
        #suggestionChip { background-color: @surfaceRaised@; border: 1px solid @borderStrong@; border-radius: 12px;
            padding: 10px 14px; color: @text@; text-align: left; }
        #suggestionChip:hover { background-color: @surfaceHover@; border-color: @accent@; color: @selectedText@; }

        #userBubble { background-color: @accentSoft@; border: 1px solid @borderStrong@; border-radius: 14px; }
        #userText { color: @text@; font-size: 13px; background: transparent; }
        #chatThumb { border: 1px solid @borderStrong@; border-radius: 6px; background: transparent; }
        #agentName { font-weight: 700; font-size: 12px; background: transparent; }
        #agentText { color: @text@; font-size: 13px; background: transparent; }
        #reasoningText, #noticeText { color: @textMuted@; font-size: 12px; background: transparent; }
        #reasoningText { font-style: italic; padding: 2px 0 4px 10px; border-left: 2px solid @borderStrong@; }
        #typingLabel { color: @textMuted@; font-size: 12px; background: transparent; }

        #codeBlock { background-color: @surface@; border: 1px solid @border@; border-radius: 10px; }
        #codeHeader { background-color: @surface@; border-bottom: 1px solid @border@;
            border-top-left-radius: 10px; border-top-right-radius: 10px; }
        #codeLanguage { color: @textMuted@; font-size: 11px; background: transparent; }
        #chatMono { background-color: @surface@; color: @text@; border: 0; padding: 6px 10px;
            border-bottom-left-radius: 10px; border-bottom-right-radius: 10px; }

        #toolCard { background-color: @surface@; border: 1px solid @border@; border-left: 3px solid @accent@;
            border-radius: 8px; }
        #toolCard[status="completed"] { border-left-color: @success@; }
        #toolCard[status="failed"] { border-left-color: @danger@; }
        #toolTitle { color: @text@; background: transparent; }
        #toolMeta { color: @textFaint@; font-size: 11px; background: transparent; }

        #chatGhost { background-color: transparent; border: 0; color: @textMuted@; padding: 2px 6px; }
        #chatGhost:hover { color: @selectedText@; background-color: @surfaceHover@; border-radius: 6px; }

        #errorCard { background-color: @dangerSoft@; border: 1px solid @dangerBorder@; border-radius: 12px; }
        #errorTitle { color: @danger@; font-weight: 700; background: transparent; }
        #errorText { color: @text@; background: transparent; }
        #commandBox { background-color: @surface@; border: 1px solid @borderStrong@; border-radius: 8px; }
        #commandText { color: @selectedText@; background: transparent; }

        #composer { background-color: @surface@; border: 1px solid @borderStrong@; border-radius: 14px; }
        #composer[focused="true"] { border-color: @accent@; }
        #composerInput { background-color: transparent; border: 0; color: @text@; font-size: 13px; padding: 4px; }
        #composerHint { color: @textFaint@; font-size: 11px; background: transparent; }
        #attachmentChip { background-color: @accentSoft@; border: 1px solid @borderStrong@; border-radius: 10px;
            color: @text@; padding: 3px 8px; }
        #composerIcon { background-color: transparent; border: 0; border-radius: 15px; font-size: 15px;
            min-width: 30px; min-height: 30px; padding: 0; }
        #composerIcon:hover { background-color: @surfaceHover@; }
        #pillPicker { background-color: @surfaceRaised@; border: 1px solid @borderStrong@; border-radius: 12px;
            padding: 4px 11px; color: @text@; font-size: 12px; }
        #pillPicker:hover { border-color: @accent@; color: @selectedText@; }
        #pillPicker::menu-indicator { image: none; width: 0; }
        #pillMenu { background-color: @surfaceRaised@; color: @text@; border: 1px solid @borderStrong@; border-radius: 8px;
            padding: 4px; }
        #pillMenu::item { padding: 6px 22px 6px 10px; border-radius: 6px; }
        #pillMenu::item:selected { background-color: @accentSoft@; color: @selectedText@; }
        #sendButton { background-color: @accentStrong@; border: 1px solid @accent@; border-radius: 17px; color: @onAccent@;
            font-size: 15px; font-weight: 700; min-width: 34px; max-width: 34px; min-height: 34px; max-height: 34px;
            padding: 0; }
        #sendButton:hover { background-color: @accent@; }
        #sendButton[running="true"] { background-color: @danger@; border-color: @danger@; }
        #sendButton:disabled { background-color: @surface@; border-color: @border@; color: @textFaint@; }
    )"), theme);
}
