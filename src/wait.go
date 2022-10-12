package main

import (
	"crypto/rand"
	"fmt"
	"log"
	"time"
)

func main() {
	b := make([]byte, 16)
	_, err := rand.Read(b)
	if err != nil {
		fmt.Println("Error: ", err)
		return
	}
	uuid := fmt.Sprintf("%X-%X-%X-%X-%X", b[0:4], b[4:6], b[6:8], b[8:10], b[10:])
	for i := 0; i < 10; i++ {
		log.Println(time.Now(), uuid)
		time.Sleep(1 * time.Second)
	}
}
