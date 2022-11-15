#include <Arduino.h>
#include "HID-Project.h"
#include "KeyboardConfig.h"
#include "Numpad.hpp"
#include "Calculator.hpp"

#include <U8g2lib.h>
#ifdef U8X8_HAVE_HW_SPI
#include <SPI.h>
#endif
#ifdef U8X8_HAVE_HW_I2C
#include <Wire.h>
#endif

// unpaged
U8G2_SSD1305_128X32_ADAFRUIT_F_4W_SW_SPI u8g2(U8G2_R0, /* clock=*/ 15, /* data=*/ 16, /* cs=*/ 10, /* dc=*/ 14, /* reset=*/ 3);
// U8G2_SSD1305_128X32_ADAFRUIT_F_4W_HW_SPI u8g2(U8G2_R0, /* cs=*/ 10, /* dc=*/ 14, /* reset=*/ 3);

// paged
// U8G2_SSD1305_128X32_ADAFRUIT_1_4W_SW_SPI u8g2(U8G2_R0, /* clock=*/ 15, /* data=*/ 16, /* cs=*/ 10, /* dc=*/ 14, /* reset=*/ 3);
// U8G2_SSD1305_128X32_ADAFRUIT_1_4W_HW_SPI u8g2(U8G2_R0, /* cs=*/ 10, /* dc=*/ 14, /* reset=*/ 3);

#define DEBUG

Numpad numpad;
Calculator calculator;

KeyboardInterface* currentMode = &numpad;

const uint8_t colPins[COLS] = {A3, A2, A1, A0};
const uint8_t rowPins[ROWS] = {9, 8, 7, 6, 5, 4};

bool tick = false;

unsigned long states[COLS][ROWS];

unsigned short longPressMs = 1000;

void setup()
{
  NKROKeyboard.begin();
  Serial.begin(9600);
  u8g2.begin();
  
  u8g2.clearBuffer();	
  u8g2.sendBuffer();	

  for (uint8_t pin : colPins) {
    pinMode(pin, INPUT_PULLUP);
  }
  
  for (uint8_t pin : rowPins) {
    pinMode(pin, OUTPUT);
    digitalWrite(pin, HIGH);
  }

  Serial.println("Ready");

  currentMode->onShow();
}

void loop()
{
  const unsigned long time = millis();

  for (uint8_t row = 0; row < ROWS; row++) {
    digitalWrite(rowPins[row], LOW);

    for (uint8_t col = 0; col < COLS; col++) {
      const unsigned long prev = states[col][row];
      const bool now = digitalRead(colPins[col]) == LOW; // todo maybe needs to be flipped?

      if (now != (prev > 0)) {
        #ifdef DEBUG
        Serial.print("["); Serial.print(row, DEC); Serial.print("]");
        Serial.print("["); Serial.print(col, DEC); Serial.print("]");
        Serial.println(now ? "DOWN" : "UP");
        #endif

        if (now) {
          states[col][row] = time;
          currentMode->onPress(row, col);
        } else {
          states[col][row] = 0;
          currentMode->onRelease(row, col);
        }
      } else if (now && (time - prev) >= longPressMs) {
        states[col][row] = time; // reset

        currentMode->onHide();

        // bad way of doing this
        // detect numlock and swich modes
        if (col == 0 && row == 1) {
          if (currentMode == &numpad) {
            currentMode = &calculator;
          } else {
            currentMode = &numpad;
          }

          currentMode->onShow();
        } else {
          currentMode->onLongPress(col, row);
        }
      }
    }

    digitalWrite(rowPins[row], HIGH);
  }

  currentMode->draw(&u8g2);
}