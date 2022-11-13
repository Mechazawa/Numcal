#include "Keyboard.h"
#include "KeyboardConfig.h"
#include "KeyboardInterface.hpp"

#pragma once


class Numpad : public KeyboardInterface {
    protected:
        bool numlock = true;

        const unsigned char mapping[2][ROWS][COLS] = {
            { // off
                {KEY_F13, KEY_F14, KEY_F15, KEY_F16},
                {KEY_NUM_LOCK, KEY_F24, KEY_F24, KEY_F24},
                {KEY_F24, KEY_UP_ARROW, KEY_F24, KEY_F24},
                {KEY_LEFT_ARROW, KEY_F24, KEY_RIGHT_ARROW, KEY_F24},
                {KEY_F24, KEY_DOWN_ARROW, KEY_F24, KEY_F24},
                {KEY_F24, KEY_F24, KEY_F24, KEY_F24},
            }, { // on
                {KEY_F13, KEY_F14, KEY_F15, KEY_F16},
                {KEY_NUM_LOCK, KEY_KP_SLASH, KEY_KP_ASTERISK, KEY_KP_MINUS},
                {KEY_KP_7, KEY_KP_8, KEY_KP_9, KEY_KP_PLUS},
                {KEY_KP_4, KEY_KP_5, KEY_KP_6, KEY_KP_PLUS},
                {KEY_KP_1, KEY_KP_2, KEY_KP_3, KEY_KP_ENTER},
                {KEY_KP_0, KEY_KP_0, KEY_KP_DOT, KEY_KP_ENTER},
            }
        };

        unsigned char getChar(unsigned char row, unsigned char column) const;

    public:
        void onPress(char row, char column) override;
        void onRelease(char row, char column) override;
};