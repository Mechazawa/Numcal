#pragma once

#define _C(x) (const unsigned char)(x)

#include "KeyboardConfig.h"
#include "KeyboardInterface.hpp"

#define CALC_VALUE_SIZE 20
#define CALC_PRECISION 4

class Calculator : public KeyboardInterface {
    protected:
        unsigned char getChar(unsigned char row, unsigned char column) const;

        const unsigned char mapping[ROWS][COLS] =  { 
            {'a', 'b', 'c', 'd'},
            {'C', '/', 'x', '-'},
            {'7', '8', '9', '+'},
            {'4', '5', '6', '+'},
            {'1', '2', '3', '\n'},
            {'0', '0', '.', '\n'},
        };

        bool drawNext = true;

        char input[CALC_VALUE_SIZE + 1];
        char result[CALC_VALUE_SIZE + 1];

        char memory[4][CALC_VALUE_SIZE] = {"0","0","0","0"};

        char pendingOperation = 0;
        char staleInput;
        bool error;
        
        void doOperation(char op);
        void doNumeric(const char input);
        void doMath(char op);
        bool hasPoint() const; 

        bool pushInput(const char value);
        bool pushResult(const char value);

        bool push(char* target, const char value, const unsigned char size = CALC_VALUE_SIZE);
    public:
        Calculator();

        void onPress(const char row, const char column) override;
        void onPress(const char input);
        void onLongPress(const char row, const char column) override;
        void onLongPress(const char input);
        void draw(U8G2* u8g2) override;
        void onShow() override;

        void loadMemory(const unsigned char slot) const;
        void storeMemory(const unsigned char slot, const char* data); 

        void clearInput();
        void clearResult();
        const char* getInput() const;
        const char* getResult() const;
};
