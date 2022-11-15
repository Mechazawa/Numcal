#pragma once

#define _C(x) (const unsigned char)(x)

#include "KeyboardConfig.h"
#include "KeyboardInterface.hpp"

// enum ECalculatorMode : unsigned char {
//     SIMPLE,
//     CYCLE_BACK,
// };

class Calculator : public KeyboardInterface {
    protected:
        // ECalculatorMode mode = ECalculatorMode::SIMPLE;

        unsigned char getChar(unsigned char row, unsigned char column) const;

        const unsigned char mapping[ROWS][COLS] =  { 
            {'a', 'b', 'c', 'd'},
            {'C', '/', '*', '-'},
            {_C(7), _C(8), _C(9), '+'},
            {_C(4), _C(5), _C(6), '+'},
            {_C(1), _C(2), _C(3), '\n'},
            {_C(0), _C(0), '.', '\n'},
        };

        // result / (10^offset)
        double result = 0; 
        long input = 0;
        unsigned char inputOffset = 0;
        bool clearNext = false;
        bool drawNext = true;

        char operation = '+';      
        double memory[4];
        
        void doOperation(char op);
        void doNumeric(char input);
        void doMath(char op);
    public:
        void onPress(char row, char column) override;
        void onLongPress(char row, char column) override;
        void draw(U8G2* u8g2) override;
        void onShow() override;

        double getInput() const;
        void setInput(double value);
        double getResult() const;

        // ECalculatorMode getMode() const;
        // void setMode(ECalculatorMode mode);
        // void nextMode();
};