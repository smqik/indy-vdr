package main

import (
	"crypto/ed25519"
	"crypto/rand"
	"encoding/json"
	"fmt"
	"log"
	"net/http"
	"os"

	"github.com/btcsuite/btcutil/base58"
	"github.com/hyperledger/indy-vdr/wrappers/golang/crypto"
	"github.com/hyperledger/indy-vdr/wrappers/golang/identifiers"
	"github.com/hyperledger/indy-vdr/wrappers/golang/vdr"
)

func main() {
	switch len(os.Args) {
	case 3:
		writeDemoTest()
	default:
		readOnlyDemo()
	}
}

// func readOnlyDemo() {
// 	genesisFile, err := http.Get("https://raw.githubusercontent.com/sovrin-foundation/sovrin/master/sovrin/pool_transactions_builder_genesis")
// 	if err != nil {
// 		log.Fatalln(err)
// 	}
// 	defer genesisFile.Body.Close()

// 	client, err := vdr.New(genesisFile.Body)
// 	if err != nil {
// 		log.Fatalln(err)
// 	}
// 	fmt.Println("Start execution of did create")
// 	client.ListDids("smahitrusteekey", "smahitrusteewallet")
// 	// client.CreateWallet("smahitrusteekey", "smahitrusteewallet")
// 	// client.DIDCreate("smahitrusteekey", "smahitrusteewallet", "smahitrustee00000000000000000000", "", "")
// 	fmt.Println("End execution of did create")
// 	//client.DID()
// 	err = client.RefreshPool()
// 	if err != nil {
// 		log.Fatalln(err)
// 	}

// 	status, err := client.GetPoolStatus()
// 	if err != nil {
// 		log.Fatalln(err)
// 	}

// 	d, _ := json.MarshalIndent(status, " ", " ")
// 	fmt.Println(string(d))

// 	rply, err := client.GetNym("FzAaV9Waa1DccDa72qwg13")
// 	if err != nil {
// 		log.Fatalln(err)
// 	}

// 	fmt.Println(rply.Data)
// }

func readOnlyDemo() {
	genesisFile, err := http.Get("https://raw.githubusercontent.com/QikHitesh/genesisfiles/refs/heads/main/pool_transactions_genesis")
	if err != nil {
		log.Fatalln(err)
	}
	defer genesisFile.Body.Close()

	client, err := vdr.New(genesisFile.Body)
	if err != nil {
		log.Fatalln(err)
	}
	// genesisFilePath := "./pool_transactions_genesis.json"
	// //file, _ := os.ReadFile(genesisFilePath)
	// file, err := os.ReadFile(genesisFilePath)
	// if err != nil {
	// 	log.Fatalln("Error in reading file: ", err)
	// }

	// // Convert []byte to io.Reader
	// reader := bytes.NewReader(file)
	// readCloser := io.NopCloser(reader)
	// client, err := vdr.New(readCloser)
	// if err != nil {
	// 	log.Fatalln("Error in creating client: ", err)
	// }
	// endorserDID := "NpK4B8mG8H3qMEXpsWXmLp"
	//privkey := ed25519.NewKeyFromSeed([]byte("hiteshsteward0000000000000000000"))
	// priv := []byte{17, 133, 76, 144, 73, 168, 92, 87, 238, 66, 22, 111, 209, 14, 83, 248, 15, 223, 74, 94, 213, 188, 242, 38, 127, 181, 127, 192, 137, 107, 33, 235, 156, 118, 230, 84, 104, 160, 63, 94, 58, 191, 158, 163, 238, 5, 222, 197, 9, 218, 67, 33, 217, 112, 24, 49, 118, 63, 28, 79, 227, 117, 125, 210}
	// privkey := ed25519.PrivateKey(priv)
	// pubKey := privkey.Public()
	// pubkey := pubKey.(ed25519.PublicKey)

	// fmt.Println("---- Private Key ---- ", base58.Encode(privkey))
	// fmt.Println("---- Public Key ---- ", base58.Encode(pubkey))
	//	pub, priv, _ := ed25519.GenerateKey()
	// sign := crypto.NewSigner(pubkey, privkey)
	// 	didDoc := `
	// 	{
	//   "@context": [
	//     "https://www.w3.org/ns/did/v1",
	//     "https://w3id.org/security/suites/ed25519-2020/v1",
	//     "https://w3id.org/security/suites/jws-2020/v1",
	//     "https://w3id.org/security/bbs/v1"
	//   ],
	//   "diddoc": {
	//     "@context": [
	//       "https://www.w3.org/ns/did/v1",
	//       "https://w3id.org/security/suites/ed25519-2020/v1",
	//       "https://w3id.org/security/suites/jws-2020/v1",
	//       "https://w3id.org/security/bbs/v1"
	//     ],
	//     "id": "did:fox:VHcJhQXWi3qDeo1x9ihJQ6",
	//     "authentication": [
	//       "did:fox:VHcJhQXWi3qDeo1x9ihJQ6#key-ed25519-1"
	//     ],
	//     "assertionMethod": [
	//       "did:fox:VHcJhQXWi3qDeo1x9ihJQ6#key-bbs-1",
	//       "did:fox:VHcJhQXWi3qDeo1x9ihJQ6#key-ed25519-1"
	//     ],
	//     "keyAgreement": [
	//       "did:fox:VHcJhQXWi3qDeo1x9ihJQ6#key-ecp384-1"
	//     ],
	//     "verificationMethod": [
	//       {
	//         "id": "did:fox:VHcJhQXWi3qDeo1x9ihJQ6#key-bbs-1",
	//         "type": "BbsBlsSignature2020",
	//         "publicKeyMultibase": "l_AgbnfnMPE0Wwnw9lixXD4vd7snyUDYUj--ADa0xN54YS8X_noPynKdoGGmIKdbD32KC7dTIPblZ_xgUvzEiBIJgMOZn0aza5lUWX2ujNEc8YV45ku_JVULiJkwwnSG"
	//       },
	//       {
	//         "id": "did:fox:VHcJhQXWi3qDeo1x9ihJQ6#key-ed25519-1",
	//         "type": "JsonWebKey2020",
	//         "publicKeyJwk": {
	//           "crv": "Ed25519",
	//           "kty": "OKP",
	//           "x": "h_j0WZuJ4JT7cXYhp-XTt48acJAhQt6n3SAsIR1xJ2E"
	//         }
	//       },
	//       {
	//         "id": "did:fox:VHcJhQXWi3qDeo1x9ihJQ6#key-ecp384-1",
	//         "type": "JsonWebKey2020",
	//         "publicKeyJwk": {
	//           "crv": "P-384",
	//           "kty": "EC",
	//           "x": "DNBQJLofrE08v-aRJikdgGcdyEWmQ3TRRAoKH_8G-8yOerhbmi_tTbDfmfxlfQmD",
	//           "y": "eI8wX1J7xWCyo3PB2foPWeOyZB0yFJJDI28d_4C8V_dKYSR7nTKIT0fA3SvopRqr"
	//         }
	//       }
	//     ]
	//   }
	// }
	// 		`
	// 	//userPrivKey := []byte{145, 56, 237, 172, 49, 117, 132, 130, 235, 132, 34, 88, 203, 121, 160, 134, 233, 37, 80, 117, 88, 23, 208, 158, 133, 21, 167, 132, 129, 206, 200, 179, 70, 167, 174, 129, 195, 203, 197, 244, 102, 119, 12, 234, 129, 11, 26, 207, 229, 198, 20, 199, 194, 181, 185, 78, 171, 196, 119, 175, 121, 225, 64, 88}
	// 	//priv := ed25519.PrivateKey(userPrivKey)
	// 	///	pub := priv.Public()
	// 	//pubBytes, _ := json.Marshal(pub)
	// 	base58encoded := "A9nDWzfkz5tEtJqbAYwiMpciYjzyQozCpibbeeTSFpgp"
	// 	fmt.Println("\n\nPub key is: ", base58encoded)
	// 	//	priv.Public()
	// 	err = client.CreateNym("VHcJhQXWi3qDeo1x9ihJQ6", base58encoded, "", endorserDID, didDoc, sign)
	// 	if err != nil {
	// 		panic(err)
	// 	}
	// //var id, name, context string
	resp, err := client.AddNewContext("")
	//resp, err := client.MultiSign()
	if err != nil {
		panic(err)
	}
	fmt.Println("\n\nResponse data is: ", resp)
	respo, err := client.AddRichSchema("")
	//resp, err := client.MultiSign()
	if err != nil {
		panic(err)
	}
	fmt.Println("\n\nResponse data is: ", respo)
	//	fmt.Println("Response is -----> ", resp)
	os.Exit(1)

	// client.AddNYM()
	os.Exit(1)

	// res, err := client.AddRichSchema("BuGZVAtnRDcQvxNYckm1CW")
	// if err != nil {
	// 	fmt.Println("Error while reading schema: ", err)
	// }
	// fmt.Println("Response is  :  ", res)
	// // _, err = client.GetRichSchema("efuihyy83yrfhuyef")
	// // if err != nil {
	// // 	fmt.Println("Error while reading schema: ", err)
	// // }
	// // err = client.RefreshPool()
	// // if err != nil {
	// // 	log.Fatalln(err)
	// // }
	// res, err := client.Submit([]byte(`{
	// 	"operation": {
	// 	"type": "1",
	// 	"dest": "LQVcTQajEfHFgC7dJeWJ6R3uBsqZrSdp9rTzv344p4A",
	// 	"verkey": "Cx5yErN9Eroiwshar79aaFUvn8BRah7i79pXjCWg1XTQ"
	// 	},
	// 	"identifier": "8yjHBnH5QL6EuBxy1cnyFvjAggHprw6R2fQYtcE3kYR2",
	// 	"endorser": "8yjHBnH5QL6EuBxy1cnyFvjAggHprw6R2fQYtcE3kYR2",
	// 	"reqId": 80345704,
	// 	"protocolVersion": 2,
	// 	"signatures": {
	// 	"8yjHBnH5QL6EuBxy1cnyFvjAggHprw6R2fQYtcE3kYR2": [
	// 	"2PCTa1TNyZaM5DBjpcDyRSTQpaC2a19Fh1U7cJQFQeeEyEaP7sdqHm1HtVfQCYDWmQefXn2gHUb5B2GxZxfiD4WR",
	// 	"4o5FAsch4sF56HMwkVZDJR9qJcTUAnBWn47k33JnsG7maSNTtnaxHEiATZMGC9C6TytVkJNajdjarkmZ1oVWvDAw"
	// 	]
	// 	}
	// 	}
	//    `))
	// if err != nil {
	// 	fmt.Println("Error is --> ", err) //
	// }
	// fmt.Println("Response is ----> ", res)
	// //	client.EndorseTransaction()
	// os.Exit(1)
	// status, err := client.GetPoolStatus()
	// if err != nil {ks
	// 	log.Fatalln("Error in fetching pool status", err)
	// }

	// d, _ := json.MarshalIndent(status, " ", " ")
	// fmt.Println(string(d))
	// e := `
	// {
	// "handle":{
	// 	"smahi":"individual"
	// }

	// }
	// `
	// reply, err := client.AddHandle("DttBXwWJjKw5yrYHCGtSqP", "myhandle")
	// if err != nil {
	// 	panic(err)
	// }
	// fmt.Println("Reply is ---> ", reply.Data)
	// reply, err := client.GetAttrib("DttBXwWJjKw5yrYHCGtSqP", "handle")
	// if err != nil {
	// 	panic(err)
	// }
	// fmt.Println("Reply is ---> ", reply.Data)
	// resp, err := client.GetNym("G7XuEvXUJ2TeJshbZ6AX5MjwuSWd27PudTW5RSmiLkzn")
	// if err != nil {
	// 	log.Fatal("Error while fetching nym transaction from ledger: ", err)
	// }
	// fmt.Println("Response received from ledger : ", resp)
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
