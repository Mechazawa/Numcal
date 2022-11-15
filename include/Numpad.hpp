#include "Arduino.h"
#include "HID-Project.h"
#include "KeyboardConfig.h"
#include "KeyboardInterface.hpp"

#pragma once


class Numpad : public KeyboardInterface {
    protected:
        unsigned char ledState = 0;
        bool drawNext = true;

    protected:
        const KeyboardKeycode mapping[ROWS][COLS] =  {
            {KEY_F13, KEY_F14, KEY_F15, KEY_F16},
            {KEY_NUM_LOCK, KEYPAD_DIVIDE, KEYPAD_MULTIPLY, KEYPAD_SUBTRACT},
            {KEYPAD_7, KEYPAD_8, KEYPAD_9, KEYPAD_ADD},
            {KEYPAD_4, KEYPAD_5, KEYPAD_6, KEYPAD_ADD},
            {KEYPAD_1, KEYPAD_2, KEYPAD_3, KEYPAD_ENTER},
            {KEYPAD_0, KEYPAD_0, KEYPAD_DOT, KEYPAD_ENTER},
        };

        KeyboardKeycode getChar(unsigned char row, unsigned char column) const;

    public:
        void onPress(char row, char column) override;
        void onRelease(char row, char column) override;
        void draw(U8G2* u8g2) override;
        void tick(const unsigned long ms) override;
        void onShow() override;
};