   10 REM TX V2.00 1 OCT 88
   20 :
   30 REM Send a file byte by byte via
   40 REM the serial port.
   50 :
   60 :
   70 CLS
   80 PRINT "Run RX on your IBM PC BEFORE pressing"
   90 PRINT "                      ******"
  100 PRINT "<RETURN> following the file name."'' 
  110 INPUT "Name of file to send ? "name$
  120 :
  130 x=OPENIN(name$)
  140 IF x=0 PRINT "File does not exist":END
  150 byte=BGET#x
  160 IF EOF#x PRINT "File is empty":END
  170 ON ERROR GOTO 440
  180 PRINT "Sending file ";name$
  190 *FX3,7
  200 *FX8,7
  210 *FX7,7
  220 *FX2,1
  230 *FX21,1
  240 *FX21,2
  250 IF INKEY(3000)<>17 THEN 540
  260 REPEAT
  270 IF INKEY(0)=19 PROC_xoff
  280 VDU byte
  290 byte=BGET#x
  300 UNTIL EOF#x
  310 VDU byte
  320 CLOSE#0
  330 *FX3,0
  340 *FX2,0
  350 VDU 7
  360 PRINT '"File ";name$;" sent to the serial port"
  370 PRINT "Press the <ESC> key on your"
  380 PRINT "MS-DOS computer to terminate"
  390 PRINT "the transfer and close the"
  400 PRINT "file."
  410 END
  420 :
  430 :
  440 ON ERROR OFF
  450 *FX3,0
  460 *FX2,0
  470 CLOSE#0
  480 PRINT:REPORT:PRINT " at line ";ERL
  490 PRINT '"Transmission aborted"
  500 END
  510 :
  520 DEF PROC_xoff
  530 IF INKEY(3000)=17 ENDPROC
  540 *FX3,0
  550 *FX2,0
  560 PRINT '"PC not ready - Program aborting"
  570 CLOSE#0
  580 END
