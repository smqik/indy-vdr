package main

import (
	"bytes"
	"crypto/ed25519"
	"crypto/rand"
	"encoding/json"
	"fmt"
	"io"
	"log"
	"net/http"
	"os"

	"github.com/hyperledger/indy-vdr/wrappers/golang/crypto"
	"github.com/hyperledger/indy-vdr/wrappers/golang/identifiers"
	"github.com/hyperledger/indy-vdr/wrappers/golang/vdr"
	"github.com/mr-tron/base58"
)

func main() {

	switch len(os.Args) {
	// case 1:
	// 	customTransaction()
	case 3:
		writeDemoTest()
	default:
		customTransaction()
	}
}

func readOnlyDemo() {
	genesisFile, err := http.Get("https://raw.githubusercontent.com/sovrin-foundation/sovrin/master/sovrin/pool_transactions_builder_genesis")
	if err != nil {
		log.Fatalln(err)
	}
	defer genesisFile.Body.Close()

	client, err := vdr.New(genesisFile.Body)
	if err != nil {
		log.Fatalln(err)
	}

	err = client.RefreshPool()
	if err != nil {
		log.Fatalln(err)
	}

	status, err := client.GetPoolStatus()
	if err != nil {
		log.Fatalln(err)
	}

	d, _ := json.MarshalIndent(status, " ", " ")
	fmt.Println(string(d))

	rply, err := client.GetNym("FzAaV9Waa1DccDa72qwg13")
	if err != nil {
		log.Fatalln(err)
	}

	fmt.Println(rply.Data)
}

func writeDemoTest() {
	genesisFile, err := http.Get("https://raw.githubusercontent.com/sovrin-foundation/sovrin/master/sovrin/pool_transactions_builder_genesis")
	if err != nil {
		log.Fatalln(err)
	}
	defer genesisFile.Body.Close()
	// genesis, err := os.Open(os.Args[1])
	// if err != nil {
	// 	log.Fatalln("unable to open genesis file", err)
	// }
	var TrusteeSeed = os.Args[2]

	client, err := vdr.New(genesisFile.Body)
	if err != nil {
		log.Fatalln(err)
	}

	err = client.RefreshPool()
	if err != nil {
		log.Fatalln(err)
	}

	status, err := client.GetPoolStatus()
	if err != nil {
		log.Fatalln(err)
	}

	d, _ := json.MarshalIndent(status, " ", " ")
	fmt.Println(string(d))

	seed, err := identifiers.ConvertSeed(TrusteeSeed[0:32])
	if err != nil {
		log.Fatalln(err)
	}

	var pubkey ed25519.PublicKey
	var privkey ed25519.PrivateKey
	privkey = ed25519.NewKeyFromSeed(seed)
	pubkey = privkey.Public().(ed25519.PublicKey)
	did, err := identifiers.CreateDID(&identifiers.MyDIDInfo{PublicKey: pubkey, Cid: true, MethodName: "fox"})
	if err != nil {
		log.Fatalln(err)
	}

	mysig := crypto.NewSigner(pubkey, privkey)

	fmt.Println("Steward DID:", did.String())
	fmt.Println("Steward Verkey:", did.Verkey)
	fmt.Println("Steward Short Verkey:", did.AbbreviateVerkey())
	someRandomPubkey, someRandomPrivkey, err := ed25519.GenerateKey(rand.Reader)
	if err != nil {
		log.Fatalln(err)
	}

	someRandomDID, err := identifiers.CreateDID(&identifiers.MyDIDInfo{PublicKey: someRandomPubkey, MethodName: "fox", Cid: true})
	if err != nil {
		log.Fatalln(err)
	}

	err = client.CreateNym(someRandomDID.DIDVal.MethodSpecificID, someRandomDID.Verkey, vdr.EndorserRole, did.DIDVal.MethodSpecificID, "", mysig)
	if err != nil {
		log.Fatalln(err)
	}
	fmt.Println("New Endorser DID:", someRandomDID.String())
	fmt.Println("New Endorser Verkey:", someRandomDID.AbbreviateVerkey())
	fmt.Println("Place These in Wallet:")
	fmt.Println("Public:", base58.Encode(someRandomPubkey))
	fmt.Println("Private:", base58.Encode(someRandomPrivkey))

	newDIDsig := crypto.NewSigner(someRandomPubkey, someRandomPrivkey)

	err = client.SetEndpoint(someRandomDID.DIDVal.MethodSpecificID, someRandomDID.DIDVal.MethodSpecificID, "http://420.69.420.69:6969", newDIDsig)
	if err != nil {
		log.Fatalln(err)
	}

	rply, err := client.GetNym(someRandomDID.DIDVal.MethodSpecificID)
	if err != nil {
		log.Fatalln(err)
	}

	fmt.Println(rply.Data)

	rply, err = client.GetEndpoint(someRandomDID.DIDVal.MethodSpecificID)
	if err != nil {
		log.Fatalln(err)
	}

	d, _ = json.MarshalIndent(rply, " ", " ")
	fmt.Println(string(d))

	//	rply, err = client.GetAuthRules()
	rply, err = client.GetTxnTypeAuthRule("1", "EDIT", "role")
	if err != nil {
		log.Fatalln(err)
	}

	d, _ = json.MarshalIndent(rply, " ", " ")
	fmt.Println(string(d))

	//rply, err = client.GetCredDef("Xy9dvEi8dkkPif5j342w9q:3:CL:23:default")
	//if err != nil {
	//	log.Fatalln(err)
	//}
	//
	//d, _ = json.MarshalIndent(rply, " ", " ")
	//fmt.Println(string(d))

	//rply, err = client.GetSchema("Xy9dvEi8dkkPif5j342w9q:2:Scoir High School:0.0.1")
	//if err != nil {
	//	log.Fatalln(err)
	//}
	//
	//d, _ = json.MarshalIndent(rply, " ", " ")
	//fmt.Println(string(d))
	//

}

func customTransaction() {
	genesisFilePath := "./pool_transactions_genesis.json"
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
	// base58string := "4xxpLJN4dwZGGZFhrMD6mKtjmg6CR81GUhwfJrvmF2zoBNArkW4rRFDc5NgK9AguC4JkjLaRenDTg9R4d7YbeSxFHfgrVwygMuE1fxm3LP6bMNNpMzYymdh6RwBHshK9FcBUKpZXajoy7vgC4e9MtFJiW7f8e99LqJmCX7PNfjUPVPyEDVu6Td2TmyfqjxfKF"
	// base58decodedstring =
	// byt := []byte{123, 34, 111, 112, 101, 114, 97, 116, 105, 111, 110, 34, 58, 123, 34, 100, 97, 116, 97, 34, 58, 123, 34, 116, 101, 115, 116, 104, 97, 110, 100, 108, 101, 49, 50, 51, 52, 34, 58, 34, 105, 110, 100, 105, 118, 105, 100, 117, 97, 108, 34, 125, 44, 34, 116, 121, 112, 101, 34, 58, 34, 57, 57, 57, 57, 52, 34, 125, 44, 34, 105, 100, 101, 110, 116, 105, 102, 105, 101, 114, 34, 58, 34, 55, 83, 116, 97, 72, 81, 83, 78, 118, 76, 87, 118, 116, 103, 80, 122, 56, 85, 66, 75, 97, 115, 84, 78, 111, 49, 90, 99, 87, 119, 71, 122, 57, 102, 107, 49, 89, 75, 90, 100, 54, 85, 117, 71, 34, 44, 34, 114, 101, 113, 73, 100, 34, 58, 52, 55, 48, 50, 49, 48, 56, 56, 52, 44, 34, 112, 114, 111, 116, 111, 99, 111, 108, 86, 101, 114, 115, 105, 111, 110, 34, 58, 50, 44, 34, 115, 105, 103, 110, 97, 116, 117, 114, 101, 34, 58, 34, 114, 103, 98, 80, 70, 107, 53, 51, 76, 74, 66, 74, 77, 67, 67, 55, 49, 74, 122, 117, 101, 66, 56, 69, 121, 120, 69, 67, 83, 103, 101, 71, 52, 85, 72, 84, 54, 113, 54, 76, 57, 69, 109, 83, 97, 68, 110, 80, 67, 72, 118, 84, 88, 72, 56, 87, 86, 88, 118, 50, 121, 99, 74, 102, 105, 68, 57, 116, 116, 113, 111, 121, 54, 97, 102, 83, 116, 121, 117, 54, 49, 89, 51, 101, 75, 83, 72, 34, 125}
	// client.Submit(byt)
	data := make(map[string]interface{})
	data["handle"] = "smqikhandle"
	_, err = client.GetHandle("BFLEtigaefTA5dvHeFhrXB", "BFLEtigaefTA5dvHeFhrXB", data)
	if err != nil {
		panic(err)
	}
}
