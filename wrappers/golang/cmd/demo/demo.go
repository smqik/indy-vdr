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
	// 	endorserDID := "BuGZVAtnRDcQvxNYckm1CW"
	// 	privkey := ed25519.NewKeyFromSeed([]byte("amitsteward000000000000000000000"))

	// 	pubkey := privkey.Public().(ed25519.PublicKey)

	// 	fmt.Println("---- Private Key ---- ", base58.Encode(privkey))
	// 	fmt.Println("---- Public Key ---- ", base58.Encode(pubkey))
	// 	//	pub, priv, _ := ed25519.GenerateKey()
	// 	sign := crypto.NewSigner(pubkey, privkey)
	// 	didDoc := `
	// 	{
	//   "@context": [
	//     "https://www.w3.org/ns/did/v1",
	//     "https://w3id.org/security/bbs/v1",
	//     "https://w3id.org/security/multikey/v1",
	//     "https://w3id.org/security/suites/jws-2020/v1",
	//     "https://w3id.org/security/v2"
	//   ],
	//   "diddoc":
	//   {
	//   "@context": [
	//     "https://www.w3.org/ns/did/v1",
	//     "https://w3id.org/security/bbs/v1",
	//     "https://w3id.org/security/multikey/v1",
	//     "https://w3id.org/security/suites/jws-2020/v1",
	//     "https://w3id.org/security/v2"
	//   ],
	//   "id": "did:fox:AmtTuiwsyTC1N2YPraPKcWSUspQuARMQDfvuC1fNR5X8",
	//   "authentication": [
	//     "did:fox:AmtTuiwsyTC1N2YPraPKcWSUspQuARMQDfvuC1fNR5X8#key-ed25519-1",
	//     "did:fox:AmtTuiwsyTC1N2YPraPKcWSUspQuARMQDfvuC1fNR5X8#key-ecp256-1",
	//     "did:fox:AmtTuiwsyTC1N2YPraPKcWSUspQuARMQDfvuC1fNR5X8#key-secp256k1-1"
	//   ],
	//   "assertionMethod": [
	//     "did:fox:AmtTuiwsyTC1N2YPraPKcWSUspQuARMQDfvuC1fNR5X8#key-bbs-1",
	//     "did:fox:AmtTuiwsyTC1N2YPraPKcWSUspQuARMQDfvuC1fNR5X8#key-ed25519-1"
	//   ],
	//   "keyAgreement": [
	//     "did:fox:AmtTuiwsyTC1N2YPraPKcWSUspQuARMQDfvuC1fNR5X8#key-x25519-1",
	//     "did:fox:AmtTuiwsyTC1N2YPraPKcWSUspQuARMQDfvuC1fNR5X8#key-ecp384-1",
	//     "did:fox:AmtTuiwsyTC1N2YPraPKcWSUspQuARMQDfvuC1fNR5X8#key-ecp256-1"
	//   ],
	//   "verificationMethod": [
	//     {
	//       "id": "did:fox:AmtTuiwsyTC1N2YPraPKcWSUspQuARMQDfvuC1fNR5X8#key-bbs-1",
	//       "type": "Multikey",
	//       "publicKeyMultibase": "lC-1irBjRgJjB7cE6jPJQAR-SBZwW7R2HcfQbAMX2ESXSJ4wisYMU984K_KepwzLEKmQEoNkXYq_R6DzsZTqSEMst8zy4O4FlDa1HAT5c6dlt7Znw9s4wMxyVpITUVgg"
	//     },
	//     {
	//       "id": "did:fox:AmtTuiwsyTC1N2YPraPKcWSUspQuARMQDfvuC1fNR5X8#key-ed25519-1",
	//       "type": "JsonWebKey2020",
	//       "publicKeyJwk": {
	//         "crv": "Ed25519",
	//         "kty": "OKP",
	//         "x": "RqeugcPLxfRmdwzqgQsaz-XGFMfCtblOq8R3r3nhQFg"
	//       }
	//     },
	//     {
	//       "id": "did:fox:AmtTuiwsyTC1N2YPraPKcWSUspQuARMQDfvuC1fNR5X8#key-x25519-1",
	//       "type": "JsonWebKey2020",
	//       "publicKeyJwk": {
	//         "crv": "X25519",
	//         "kty": "OKP",
	//         "x": "hoLYb-L8VZlq__w8PvLE3rxrNiWxRbPHXg6KmM4iUFQ"
	//       }
	//     },
	//     {
	//       "id": "did:fox:AmtTuiwsyTC1N2YPraPKcWSUspQuARMQDfvuC1fNR5X8#key-ecp384-1",
	//       "type": "JsonWebKey2020",
	//       "publicKeyJwk": {
	//         "crv": "P-384",
	//         "kty": "EC",
	//         "x": "Xy2AzxYLHRmQnWPFIwVuRGv6zF8ee0e8VrH1vXNQeklR0hLhRFemlHtKqnlvMB69",
	//         "y": "5sENVXeKBVzSdE79BFyP2_6vnjUoIWRsdqEcJqeuCoDMjPqEEjKtECFYiCzRkLNn"
	//       }
	//     },
	//     {
	//       "id": "did:fox:AmtTuiwsyTC1N2YPraPKcWSUspQuARMQDfvuC1fNR5X8#key-ecp256-1",
	//       "type": "JsonWebKey2020",
	//       "publicKeyJwk": {
	//         "crv": "P-256",
	//         "kty": "EC",
	//         "x": "UN4nq8lSSe2YZU1XuVdtE1MvdMIIMcf44YKBMERin9U",
	//         "y": "HWKmQ4_feca5rUvGBkYjTZ4ikHKc3F_yTi3ceUeBmHo"
	//       }
	//     },
	//     {
	//       "id": "did:fox:AmtTuiwsyTC1N2YPraPKcWSUspQuARMQDfvuC1fNR5X8#key-secp256k1-1",
	//       "type": "JsonWebKey2020",
	//       "publicKeyJwk": {
	//         "crv": "secp256k1",
	//         "kty": "EC",
	//         "x": "s7nt3iVxjWIiGpWwkYTAWz_wrQYLV6qqo1C8QGZFLjM",
	//         "y": "aYltKH0PKYnCBhFWsRHuOb05TGL6DKAEA5bYTyRaTQg"
	//       }
	//     }
	//   ],
	//   "service": [
	//     {
	//       "id": "did:fox:AmtTuiwsyTC1N2YPraPKcWSUspQuARMQDfvuC1fNR5X8#didcomm-1",
	//       "type": "DIDCommMessaging",
	//       "serviceEndpoint": {
	//         "accept": [
	//           "didcomm/v2",
	//           "didcomm/aip2;env=rfc587"
	//         ],
	//         "uri": "https://authorizationserver-dot-qikfox-identity-ecosystem.wl.r.appspot.com"
	//       }
	//     }
	//   ]
	// }
	// }
	// 	`
	// 	//userPrivKey := []byte{145, 56, 237, 172, 49, 117, 132, 130, 235, 132, 34, 88, 203, 121, 160, 134, 233, 37, 80, 117, 88, 23, 208, 158, 133, 21, 167, 132, 129, 206, 200, 179, 70, 167, 174, 129, 195, 203, 197, 244, 102, 119, 12, 234, 129, 11, 26, 207, 229, 198, 20, 199, 194, 181, 185, 78, 171, 196, 119, 175, 121, 225, 64, 88}
	// 	//priv := ed25519.PrivateKey(userPrivKey)
	// 	///	pub := priv.Public()
	// 	//pubBytes, _ := json.Marshal(pub)
	// 	base58encoded := "5koqZt1PHyWc2xnEhL6pbqJovJLuyZddKQyFNUwYEVeF"
	// 	fmt.Println("\n\nPub key is: ", base58encoded)
	// 	//	priv.Public()
	//err = client.CreateNym("AmtTuiwsyTC1N2YPraPKcWSUspQuARMQDfvuC1fNR5X8", base58encoded, "", endorserDID, didDoc, sign)

	//var id, name, context string
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

	// // client.AddNYM()
	// os.Exit(1)

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
