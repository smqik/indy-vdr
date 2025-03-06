package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"log"
	"os"

	"github.com/hyperledger/indy-vdr/wrappers/golang/vdr"
)

func Gateway() {
	genesisFilePath := "D:/go-work/src/blockchaingatewayservice/blockchaingateway/genesis/pool_transactions_genesis"
	//file, _ := os.ReadFile(genesisFilePath)
	file, err := os.ReadFile(genesisFilePath)
	if err != nil {
		log.Fatalln("Error in reading file: ", err)
	}

	// Convert []byte to io.Reader
	reader := bytes.NewReader(file)
	readCloser := io.NopCloser(reader)
	client, err := vdr.New(readCloser)
	if err != nil {
		log.Fatalln("Error in creating client: ", err)
	}

	// err = client.RefreshPool()
	// if err != nil {
	// 	log.Fatalln(err)
	// }

	status, err := client.GetPoolStatus()
	if err != nil {
		log.Fatalln("Error in fetching pool status", err)
	}

	d, _ := json.MarshalIndent(status, " ", " ")
	fmt.Println(string(d))

	resp, err := client.GetNym("9czhdddKFttRT43YwmyAsy")
	if err != nil {
		log.Fatal("Error while fetching nym transaction from ledger: ", err)
	}
	fmt.Println("Response received from ledger : ", resp.Data)
}

// import (
// 	"github.com/hyperledger/indy-vdr/wrappers/golang/vdr"
// )

// func did() {
// 	vdr.D()
// }

// // import (
// 	"encoding/json"
// 	"fmt"
// 	"log"
// 	"net/http"

// 	"github.com/hyperledger/indy-vdr/wrappers/golang/vdr"
// )

// // func main() {

// // 	switch len(os.Args) {
// // 	case 3:
// // 		writeDemoTest()
// // 	default:
// // 		readOnlyDemo()
// // 	}
// // }

// func addNewDIDDemo() {
// 	genesisFile, err := http.Get("https://raw.githubusercontent.com/sovrin-foundation/sovrin/master/sovrin/pool_transactions_builder_genesis")
// 	if err != nil {
// 		log.Fatalln(err)
// 	}
// 	defer genesisFile.Body.Close()

// }
