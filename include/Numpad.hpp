#include "Arduino.h"
#include "HID-Project.h"
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
                {KEY_NUM_LOCK, HID_KEYPAD_DIVIDE, HID_KEYPAD_MULTIPLY, HID_KEYPAD_SUBTRACT},
                {HID_KEYPAD_7_AND_HOME, HID_KEYPAD_8_AND_UP_ARROW, HID_KEYPAD_9_AND_PAGE_UP, HID_KEYPAD_ADD},
                {HID_KEYPAD_4_AND_LEFT_ARROW, HID_KEYPAD_5, HID_KEYPAD_6_AND_RIGHT_ARROW, HID_KEYPAD_ADD},
                {HID_KEYPAD_1_AND_END, HID_KEYPAD_2_AND_DOWN_ARROW, HID_KEYPAD_3_AND_PAGE_DOWN, HID_KEYPAD_ENTER},
                {HID_KEYPAD_0_AND_INSERT, HID_KEYPAD_0_AND_INSERT, HID_KEYPAD_DECIMAL, HID_KEYPAD_ENTER},
            }
        };

        unsigned char getChar(unsigned char row, unsigned char column) const;

    public:
        void onPress(char row, char column) override;
        void onRelease(char row, char column) override;
};